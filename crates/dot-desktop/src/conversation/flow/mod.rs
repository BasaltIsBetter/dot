mod pane;
mod question;
mod reply;
mod tool;

use std::time::Instant;

use dot_api::{Image, Prompt, Text, Translator};
use winit::dpi::PhysicalPosition;

use crate::canvas::{Glide, Layout, Pane, Scene, to_f32};
use crate::conversation::flow::question::Question;
use crate::conversation::prompter::{Attachment, Edit, Intent, Prompter};
use crate::conversation::transcript::{Transcript, lines};
use crate::conversation::zone::{Area, TextWidth, Zones};
use crate::event::Wheel;

const ROWS_PER_WHEEL_LINE: f64 = 3.0;
const KIBIBYTE: usize = 1_024;
const MEBIBYTE: usize = KIBIBYTE * KIBIBYTE;
const TENTHS: usize = 10;

#[derive(Debug, Default)]
pub(crate) struct Conversation {
    prompter: Prompter,
    transcript: Transcript,
    question: Option<Question>,
    glide: Option<Glide>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Ask(Prompt),
    Stop,
}

impl Conversation {
    pub(crate) fn type_key(
        &mut self,
        intent: Intent,
        layout: &Layout,
        now: Instant,
        clipboard: impl FnOnce() -> Option<String>,
    ) -> Option<Command> {
        match intent {
            Intent::Edit(edit) => self.edit(edit, layout),
            Intent::Send => return self.send(layout, now).map(Command::Ask),
            Intent::Stop => return self.transcript.is_live().then_some(Command::Stop),
            Intent::Paste => {
                if let Some(text) = clipboard() {
                    self.edit(Edit::Insert(text), layout);
                }
            }
        }
        None
    }

    pub(crate) fn attach(&mut self, name: &str, image: Image, translator: &impl Translator) {
        self.prompter.attach(Attachment {
            kind: translator.translate(Text::ImageLabel).into_owned(),
            name: name.to_owned(),
            size: size(image.bytes().len(), translator),
            image,
        });
    }

    pub(crate) fn notice(&mut self, text: &str, now: Instant) {
        self.transcript.notice(text, now);
    }

    pub(crate) fn commit(&mut self, text: String, layout: &Layout) {
        self.edit(Edit::Insert(text), layout);
    }

    pub(crate) fn compose(&mut self, text: String) {
        if self.question.is_none() {
            self.prompter.compose(text);
        }
    }

    pub(crate) fn fit(&mut self, layout: &Layout) -> bool {
        let width = TextWidth::of(layout);
        if self.prompter.width() == width {
            return false;
        }

        self.prompter.rewrap_to(width);
        if let Some(question) = self.question.as_mut() {
            question.rewrap_to(width);
        }
        let zones = self.zones(layout);
        self.prompter.reveal_caret(zones.prompter_rows);
        self.transcript
            .rewrap(width, zones.transcript.rows(layout.line_height()));
        true
    }

    pub(crate) fn scroll(
        &mut self,
        wheel: Wheel,
        at: PhysicalPosition<f64>,
        layout: &Layout,
        now: Instant,
    ) {
        let zones = self.zones(layout);
        let line_height = layout.line_height();
        let rows_up = match wheel {
            Wheel::Lines(lines) => f64::from(lines) * ROWS_PER_WHEEL_LINE,
            Wheel::Pixels(pixels) => pixels / f64::from(line_height),
        };

        match zones.area_at(to_f32(at.y)) {
            Some(Area::Transcript) => {
                let visible = zones.transcript.rows(line_height);
                self.transcript.scroll_by(rows_up, visible, now);
            }
            Some(Area::Prompter) => match self.question.as_mut() {
                Some(question) => question.scroll_by(rows_up, zones.prompter_rows),
                None => self.prompter.scroll_by(rows_up, zones.prompter_rows),
            },
            None => {}
        }
    }

    pub(crate) fn press(&mut self, at: PhysicalPosition<f64>, layout: &Layout) {
        let zones = self.zones(layout);
        let point = [to_f32(at.x) - self.settled_shift(layout), to_f32(at.y)];
        if self.question.is_none() && zones.area_at(point[1]) == Some(Area::Prompter) {
            self.prompter
                .place(point, zones.prompter, zones.prompter_rows, layout);
        }
    }

    pub(crate) fn scene(
        &self,
        layout: &Layout,
        now: Instant,
        focused: bool,
        translator: &impl Translator,
    ) -> Scene {
        let zones = self.zones(layout);
        let transcript = Pane {
            top: zones.transcript.top,
            bottom: zones.transcript.bottom,
            shift: 0.0,
            lines: lines(&self.transcript, zones.transcript, layout, now),
        };
        let prompter = self.prompter_pane(zones, layout, now, focused, translator);
        Scene {
            panes: vec![transcript, prompter],
            animating: self.transcript.is_animating(now)
                || self.glide.is_some_and(|glide| glide.is_moving(now)),
            ..Scene::default()
        }
    }

    fn edit(&mut self, edit: Edit, layout: &Layout) {
        if self.question.is_some() {
            return;
        }
        self.prompter.edit(edit);
        let zones = self.zones(layout);
        self.prompter.reveal_caret(zones.prompter_rows);
        self.transcript
            .refit(zones.transcript.rows(layout.line_height()));
    }

    fn send(&mut self, layout: &Layout, now: Instant) -> Option<Prompt> {
        if self.transcript.is_live() {
            return None;
        }
        let width = self.prompter.width();
        let (text, attachments) = self.prompter.take()?;
        self.transcript.append(&text, now);
        let images = attachments
            .into_iter()
            .map(|attachment| {
                self.transcript.attach(&attachment.label(width), now);
                attachment.image
            })
            .collect();
        let visible = self.visible_rows(layout);
        self.transcript.begin_reply(visible, now);
        Some(Prompt { text, images })
    }

    fn visible_rows(&self, layout: &Layout) -> f64 {
        self.zones(layout).transcript.rows(layout.line_height())
    }

    fn zones(&self, layout: &Layout) -> Zones {
        let rows = self
            .question
            .as_ref()
            .map_or_else(|| self.prompter.rows(), Question::rows);
        Zones::new(layout, rows)
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
