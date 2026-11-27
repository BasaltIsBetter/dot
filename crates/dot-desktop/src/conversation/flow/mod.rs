mod activity;
mod below;
mod frame;
mod pause;
mod question;
mod recall;
mod replay;
mod reply;
mod running;
mod select;

pub(crate) use self::recall::{Recall, Recalling};
pub(crate) use self::reply::revealed;
pub(in crate::conversation) use self::reply::{name_of, summary};

use std::time::Instant;

use dot_api::{Image, ImageError, ImageLabel, Opening, Prompt, Text, Translator};
use winit::dpi::PhysicalPosition;

use self::reply::Pacer;
use self::running::Marker;
use self::select::Selecting;
use crate::canvas::{Layout, Silhouette};
use crate::conversation::agent::{AgentView, Pulse};
use crate::conversation::blink::Blink;
use crate::conversation::flow::frame::Help;
use crate::conversation::flow::pause::{Pause, Queued};
use crate::conversation::flow::question::Question;
use crate::conversation::instrument::Instrument;
use crate::conversation::prompter::{Attachment, Edit, Intent, Prompter};
use crate::conversation::stage::{Stage, text_width};
use crate::conversation::suggestion::Suggestions;
use crate::conversation::tint::Colours;
use crate::editor::Touched;
use crate::event::Wheel;
use crate::reply::Opened;
use dot_transcript::{Agents, Reader, Transcript, index, to_f32};

const ROWS_PER_WHEEL_LINE: f64 = 3.0;
const KIBIBYTE: usize = 1_024;
const MEBIBYTE: usize = KIBIBYTE * KIBIBYTE;
const TENTHS: usize = 10;

#[derive(Debug, Default)]
pub(crate) struct Conversation {
    prompter: Prompter,
    transcript: Transcript,
    reader: Reader,
    question: Option<Question>,
    suggestions: Suggestions,
    blink: Blink,
    help: Help,
    instrument: Instrument,
    agents: Agents,
    agent_view: Option<AgentView>,
    pulse: Pulse,
    marker: Marker,
    silhouette: Silhouette,
    back_from_agent: Option<Instant>,
    pause: Option<Pause>,
    queued: Option<Queued>,
    writing: bool,
    replied: bool,
    pointer: Option<[f32; 2]>,
    touched: Touched,
    selecting: Option<Selecting>,
    colours: Colours,
    pacer: Option<Pacer>,
    opened: Option<Opened>,
    recalling: Option<Recalling>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Ask(Prompt),
    Redirect(Prompt),
    Stop,
    StartOver,
}

impl Conversation {
    pub(crate) fn type_key(
        &mut self,
        intent: Intent,
        now: Instant,
        clipboard: impl FnOnce() -> Option<String>,
        translator: &impl Translator,
    ) -> Option<Command> {
        self.blink.restart(now);
        self.help.stir(now);
        if !matches!(intent, Intent::Edit(Edit::Move(_))) {
            self.recalling = None;
        }
        if self.agent_view.is_some() {
            if intent == Intent::Stop {
                self.close_agent(now);
            }
            return None;
        }
        match intent {
            Intent::Edit(edit) => self.edit(edit, now),
            Intent::Send if self.pause.is_some() => return self.resume(now, translator),
            Intent::Send if self.transcript.is_live() => {
                if let Err(error) = self.queue() {
                    let notice = translator.translate(error.text());
                    self.notice(&notice, now);
                }
            }
            Intent::Send => return self.send(now).map(Command::Ask),
            Intent::Pause if self.pause.is_none() => self.pause(),
            Intent::Pause if self.prompter.is_empty() => return self.resume(now, translator),
            Intent::Pause | Intent::Switch(_) => {}
            Intent::Stop if self.pause.is_some() => return self.unpause_to_stop(now, translator),
            Intent::Stop => return self.transcript.is_live().then_some(Command::Stop),
            Intent::StartOver => return (!self.transcript.is_live()).then_some(Command::StartOver),
            Intent::Paste => {
                if let Some(text) = clipboard() {
                    self.edit(Edit::Insert(text), now);
                }
            }
        }
        None
    }

    pub(crate) fn attach(
        &mut self,
        name: &str,
        image: Image,
        translator: &impl Translator,
    ) -> Result<(), ImageError> {
        self.prompter.attach(Attachment {
            kind: translator.translate(Text::ImageLabel).into_owned(),
            name: name.to_owned(),
            size: size(image.bytes().len(), translator),
            image,
        })
    }

    pub(crate) fn notice(&mut self, text: &str, now: Instant) {
        self.transcript.notice(text, now);
    }

    pub(crate) fn commit(&mut self, text: String, now: Instant) {
        self.edit(Edit::Insert(text), now);
    }

    pub(crate) fn compose(&mut self, text: String) {
        if self.question.is_none() {
            self.prompter.compose(text);
        }
    }

    pub(crate) fn fit(&mut self, layout: &Layout, now: Instant) -> bool {
        let width = text_width(layout);
        if self.prompter.width() == width {
            return false;
        }
        self.prompter.rewrap_to(width);
        if let Some(pause) = self.pause.as_mut() {
            pause.draft.rewrap_to(width);
        }
        if let Some(question) = self.question.as_mut() {
            question.rewrap_to(width);
        }
        let focus = self.reader.focus(now);
        let kept = self.transcript.rewrap(width, focus, now);
        self.reader.land_on(kept);
        true
    }

    pub(crate) fn scroll(
        &mut self,
        wheel: Wheel,
        (at, layout): (PhysicalPosition<f64>, &Layout),
        now: Instant,
        translator: &impl Translator,
    ) {
        let stage = Stage::of(layout);
        if to_f32(at.y) >= stage.floor {
            return;
        }
        let rows_up = match wheel {
            Wheel::Lines(lines) => f64::from(lines) * ROWS_PER_WHEEL_LINE,
            Wheel::Pixels(pixels) => pixels / f64::from(stage.row),
        };
        if self.agent_view.is_some() {
            self.scroll_agent(rows_up, now, translator);
            return;
        }
        self.reader.scroll_by(-rows_up);
        self.help.stir(now);
    }

    pub(crate) const fn point(&mut self, at: Option<[f32; 2]>) {
        self.pointer = at;
    }

    pub(crate) fn press(&mut self, at: PhysicalPosition<f64>, layout: &Layout, now: Instant) {
        if self.question.is_some() || self.agent_view.is_some() {
            return;
        }
        let stage = Stage::of(layout);
        let Some(row) = stage.row_at(to_f32(at.y), self.reader.focus(now)) else {
            return;
        };
        let lane = self
            .below(now)
            .running
            .and_then(|first| row.checked_sub(first))
            .and_then(|row| self.lane_at(row, now))
            .map(str::to_owned);
        if let Some(id) = lane {
            self.open_agent(&id, now);
            return;
        }
        let column = f64::from((to_f32(at.x) - stage.column[0]) / layout.advance());
        if self
            .transcript
            .toggle_code(row, index(column.floor().max(0.0)))
        {
            return;
        }
        let Some(below) = row.checked_sub(self.editing_row(now)) else {
            return;
        };
        self.prompter.place(below, index(column.round()));
        self.reader.come_back();
    }

    fn edit(&mut self, edit: Edit, now: Instant) {
        if self.question.is_some() {
            return;
        }
        if !matches!(edit, Edit::Move(_)) {
            self.recalling = None;
        }
        self.prompter.edit(edit, now);
        self.reader.come_back();
    }

    fn send(&mut self, now: Instant) -> Option<Prompt> {
        let (text, attachments) = self.prompter.take()?;
        Some(self.open(Opening::Prompt, text, attachments, now))
    }

    fn open(
        &mut self,
        opening: Opening,
        text: String,
        attachments: Vec<Attachment>,
        now: Instant,
    ) -> Prompt {
        let width = self.prompter.width();
        let labels: Vec<String> = attachments
            .iter()
            .map(|attachment| attachment.label(width))
            .collect();
        self.show_opening(opening, &text, &labels, now);
        let (images, named) = attachments
            .into_iter()
            .map(|attachment| {
                let bytes = u64::try_from(attachment.image.bytes().len()).unwrap_or(u64::MAX);
                let named = ImageLabel {
                    name: attachment.name,
                    bytes,
                };
                (attachment.image, named)
            })
            .unzip();
        self.opened = Some(Opened {
            opening,
            images: named,
        });
        Prompt { text, images }
    }

    pub(crate) fn opened(&self) -> Opened {
        self.opened.clone().unwrap_or_default()
    }

    fn show_opening(&mut self, opening: Opening, text: &str, labels: &[String], now: Instant) {
        match opening {
            Opening::Prompt => self.transcript.append(text, now),
            Opening::Note | Opening::Redirect => self.transcript.note(text, now),
        }
        for label in labels {
            self.transcript.attach(label, now);
        }
        self.transcript.begin_reply(now);
        let exchange = self.transcript.exchanges().saturating_sub(1);
        self.instrument.start(exchange, now);
        self.writing = false;
        self.reader.come_back();
        self.replied = true;
    }
}

fn size(bytes: usize, translator: &impl Translator) -> String {
    if bytes < MEBIBYTE {
        let kibibytes = bytes.div_ceil(KIBIBYTE);
        return format!("{kibibytes} {}", translator.translate(Text::SizeKilobytes));
    }
    let tenths = (bytes * TENTHS + MEBIBYTE / 2) / MEBIBYTE;
    format!(
        "{}.{} {}",
        tenths / TENTHS,
        tenths % TENTHS,
        translator.translate(Text::SizeMegabytes)
    )
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/mod.rs"]
mod tests;
