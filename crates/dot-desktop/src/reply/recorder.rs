use std::borrow::Cow;
use std::time::{Duration, Instant};

use dot_api::{
    ArchiveError, Asked, Chat, Ending, Failure, Happened, ImageLabel, Journal, Opening, Prompt,
    Record, Reply, ReplyEvent,
};

use crate::error::Problem;

const MERGED_FOR: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Opened {
    pub(crate) opening: Opening,
    pub(crate) images: Vec<ImageLabel>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Clock {
    origin: Option<Instant>,
    carried: Duration,
}

pub(crate) struct Recorder<J> {
    journal: J,
    clock: Clock,
    piece: Option<Piece>,
    failing: bool,
}

struct Piece {
    thinking: bool,
    text: String,
    at: Instant,
}

impl Default for Opened {
    fn default() -> Self {
        Self {
            opening: Opening::Prompt,
            images: Vec::new(),
        }
    }
}

impl Clock {
    pub(crate) const fn resuming_at(carried: Duration) -> Self {
        Self {
            origin: None,
            carried,
        }
    }

    pub(crate) fn at(&mut self, now: Instant) -> Duration {
        let origin = *self.origin.get_or_insert(now);
        self.carried
            .saturating_add(now.saturating_duration_since(origin))
    }
}

impl<J: Journal> Recorder<J> {
    pub(crate) const fn new(journal: J, clock: Clock) -> Self {
        Self {
            journal,
            clock,
            piece: None,
            failing: false,
        }
    }

    pub(crate) fn asked(&mut self, prompt: &Prompt, opened: Opened, now: Instant) {
        let asked = Asked {
            text: Cow::Borrowed(&prompt.text),
            opening: opened.opening,
            images: opened.images,
        };
        self.write(Happened::Asked(asked), now);
    }

    pub(crate) fn heard(&mut self, event: &ReplyEvent, now: Instant) {
        let (thinking, piece) = match event {
            ReplyEvent::Text(piece) => (false, piece),
            ReplyEvent::Thinking(piece) => (true, piece),
            ReplyEvent::ToolOutput { .. } => return,
            other => {
                self.write_the_piece();
                self.write(Happened::Heard(Cow::Borrowed(other)), now);
                return;
            }
        };
        if let Some(merged) = self.piece.as_mut()
            && merged.thinking == thinking
            && now.saturating_duration_since(merged.at) < MERGED_FOR
        {
            merged.text.push_str(piece);
            return;
        }
        self.write_the_piece();
        self.piece = Some(Piece {
            thinking,
            text: piece.clone(),
            at: now,
        });
    }

    pub(crate) fn ended(&mut self, ended: &Result<Reply, Problem>, spent: Reply, now: Instant) {
        self.write_the_piece();
        let failure = ended.as_ref().err().map(|problem| Failure {
            text: problem.text,
            cause: problem.cause.clone(),
        });
        self.write(Happened::Ended(Ending { spent, failure }), now);
        self.flush();
    }

    pub(crate) fn started_over(&mut self, now: Instant) {
        self.write_the_piece();
        self.write(Happened::StartedOver, now);
        self.flush();
    }

    pub(crate) fn remember(&mut self, chat: &impl Chat) {
        let kept = self
            .journal
            .keep_memory(&|memory: &mut dyn std::io::Write| chat.remember(memory));
        if let Err(error) = kept {
            self.warn(&error);
        }
    }

    fn write_the_piece(&mut self) {
        let Some(Piece { thinking, text, at }) = self.piece.take() else {
            return;
        };
        let event = if thinking {
            ReplyEvent::Thinking(text)
        } else {
            ReplyEvent::Text(text)
        };
        self.write(Happened::Heard(Cow::Owned(event)), at);
    }

    fn write(&mut self, happened: Happened<'_>, now: Instant) {
        let record = Record {
            at: self.clock.at(now),
            happened,
        };
        match self.journal.write(&record) {
            Ok(()) => self.failing = false,
            Err(error) => self.warn(&error),
        }
    }

    fn flush(&mut self) {
        if let Err(error) = self.journal.flush() {
            self.warn(&error);
        }
    }

    fn warn(&mut self, error: &ArchiveError) {
        if !self.failing {
            tracing::warn!(%error, "the session is not kept");
        }
        self.failing = true;
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/reply/recorder.rs"]
pub(crate) mod tests;
