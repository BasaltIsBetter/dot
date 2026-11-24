use std::cell::Cell;
use std::mem;
use std::time::Instant;

use dot_api::{ImageError, Prompt, ReplyEvent, Translator};

use crate::conversation::flow::{Command, Conversation, Opening};
use crate::conversation::prompter::{Attachment, Prompter};
use dot_transcript::{TextWidth, wrap};

#[derive(Debug, Default)]
pub(crate) struct Pause {
    pub(super) draft: Prompter,
    held: Vec<ReplyEvent>,
    ended: Option<Ended>,
}

#[derive(Debug, Default)]
pub(crate) struct Queued {
    pub(super) text: String,
    pub(super) attachments: Vec<Attachment>,
    pub(super) rows: Cell<Option<(TextWidth, usize)>>,
}

enum Resumed {
    Without,
    With(String, Vec<Attachment>),
}

#[derive(Debug)]
struct Ended {
    notice: Option<String>,
    written: Option<u64>,
}

impl Queued {
    pub(super) fn rows_at(&self, width: TextWidth) -> usize {
        if let Some((known, rows)) = self.rows.get()
            && known == width
        {
            return rows;
        }
        let rows = wrap(&self.text, width.columns()).len() + self.attachments.len();
        self.rows.set(Some((width, rows)));
        rows
    }
}

impl Conversation {
    pub(super) fn pause(&mut self) {
        if !self.transcript.is_live() || self.pause.is_some() {
            return;
        }
        let width = self.prompter.width();
        let mut note = Prompter::default();
        note.rewrap_to(width);
        let draft = mem::replace(&mut self.prompter, note);
        self.pause = Some(Pause {
            draft,
            ..Pause::default()
        });
        self.reader.come_back();
    }

    pub(super) fn hold(&mut self, event: ReplyEvent) -> Option<ReplyEvent> {
        let Some(pause) = self.pause.as_mut() else {
            return Some(event);
        };
        pause.held.push(event);
        None
    }

    pub(super) fn hold_the_end(&mut self, notice: Option<&str>, written: Option<u64>) -> bool {
        let Some(pause) = self.pause.as_mut() else {
            return false;
        };
        pause.ended = Some(Ended {
            notice: notice.map(str::to_owned),
            written,
        });
        true
    }

    pub(super) fn resume(&mut self, now: Instant, translator: &impl Translator) -> Option<Command> {
        let Resumed::With(text, attachments) = self.release(now, translator)? else {
            return self.finished_reply_sends_the_queue(now);
        };
        if self.transcript.is_live() {
            self.end_without_a_word(now);
            let prompt = self.open(Opening::Note, text, attachments, now);
            return Some(Command::Redirect(prompt));
        }
        let prompt = self.open(Opening::Note, text, attachments, now);
        Some(Command::Ask(prompt))
    }

    pub(super) fn unpause_to_stop(
        &mut self,
        now: Instant,
        translator: &impl Translator,
    ) -> Option<Command> {
        self.release(now, translator)?;
        if self.transcript.is_live() {
            return Some(Command::Stop);
        }
        self.finished_reply_sends_the_queue(now)
    }

    pub(super) fn queue(&mut self) -> Result<(), ImageError> {
        if let Some(queued) = &self.queued {
            self.prompter.fits_after(&queued.attachments)?;
        }
        let Some((text, attachments)) = self.prompter.take() else {
            return Ok(());
        };
        let queued = self.queued.get_or_insert_with(Queued::default);
        if !queued.text.is_empty() && !text.is_empty() {
            queued.text.push('\n');
        }
        queued.text.push_str(&text);
        queued.attachments.extend(attachments);
        queued.rows.set(None);
        Ok(())
    }

    pub(crate) fn send_the_queue(&mut self, now: Instant) -> Option<Prompt> {
        if self.transcript.is_live() || self.pause.is_some() {
            return None;
        }
        let Queued {
            text, attachments, ..
        } = self.queued.take()?;
        Some(self.open(Opening::Prompt, text, attachments, now))
    }

    fn release(&mut self, now: Instant, translator: &impl Translator) -> Option<Resumed> {
        let pause = self.pause.take()?;
        let note = match mem::replace(&mut self.prompter, pause.draft).take() {
            Some((text, attachments)) => Resumed::With(text, attachments),
            None => Resumed::Without,
        };
        self.show_what_waits(now, translator);
        for event in pause.held {
            self.apply(event, now, translator);
        }
        if let Some(Ended { notice, written }) = pause.ended {
            self.finish_reply(notice.as_deref(), written, now);
        }
        Some(note)
    }

    fn finished_reply_sends_the_queue(&mut self, now: Instant) -> Option<Command> {
        self.send_the_queue(now).map(Command::Ask)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/pause.rs"]
mod tests;
