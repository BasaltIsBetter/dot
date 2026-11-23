mod reply;
mod rows;
mod tool;
mod veil;

use std::ops::Range;
use std::time::Instant;

use crate::line::TextWidth;
use crate::motion::SETTLE;
use crate::transcript::entry::{Entry, Kind};
use crate::transcript::fade::is_fresh;
use crate::transcript::ghost::Ghost;

/// The conversation as it is shown: every entry, wrapped, the reply being written over the
/// question it answers, and the tools Claude calls along the way.
#[derive(Debug, Default)]
pub struct Transcript {
    entries: Vec<Entry>,
    prompts: Vec<usize>,
    last_prompt: Option<usize>,
    ends: Vec<usize>,
    gaps: Vec<bool>,
    width: TextWidth,
    ghost: Option<Ghost>,
    reply: Reply,
    reply_start: usize,
    running_tools: Vec<RunningTool>,
    returned: Option<Returned>,
    insertions: Vec<Insertion>,
    last_arrival: Option<Instant>,
    code_until: Option<Instant>,
    settled: usize,
}

/// Rows added above rows already shown: whatever was at or past row `at` moved down `rows` rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Insertion {
    /// The first row that moved.
    pub at: f64,
    /// How many rows it moved down.
    pub rows: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Reply {
    #[default]
    Idle,
    Waiting,
    Writing,
}

#[derive(Debug)]
struct RunningTool {
    id: String,
    position: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct Returned {
    question: Range<usize>,
    at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row<'a> {
    Gap,
    Text {
        entry: &'a Entry,
        row: usize,
        position: usize,
    },
}

impl Transcript {
    /// Adds what the user sent.
    pub fn append(&mut self, text: &str, now: Instant) {
        self.last_prompt = Some(self.entries.len());
        self.prompts.push(self.entries.len());
        self.push(Entry::new(Kind::Prompt, text, now, self.width), now);
    }

    /// Adds a note the user slipped into the reply, which opens an exchange of its own; the
    /// reply that answers it is not written over it.
    pub fn note(&mut self, text: &str, now: Instant) {
        self.prompts.push(self.entries.len());
        self.push(Entry::new(Kind::Note, text, now, self.width), now);
    }

    /// Adds the row of an image sent with the last prompt, under it.
    pub fn attach(&mut self, label: &str, now: Instant) {
        self.push(Entry::new(Kind::Attachment, label, now, self.width), now);
    }

    /// Adds a notice outside a reply.
    pub fn notice(&mut self, text: &str, now: Instant) {
        self.push(Entry::new(Kind::Notice, text, now, self.width), now);
    }

    /// Hands over the rows added above rows already shown since the last call, oldest first,
    /// for whoever keeps the view still to move along.
    pub fn take_insertions(&mut self) -> Vec<Insertion> {
        std::mem::take(&mut self.insertions)
    }

    pub(crate) const fn width(&self) -> TextWidth {
        self.width
    }

    pub(crate) fn entry(&self, position: usize) -> Option<&Entry> {
        self.entries.get(position)
    }

    /// Returns whether the last text that arrived is still cooling from orange at `now`.
    #[must_use]
    pub fn is_fresh(&self, now: Instant) -> bool {
        self.last_arrival
            .is_some_and(|arrival| is_fresh(now.saturating_duration_since(arrival)))
    }

    /// Returns whether anything still moves, fades or cools at `now`, so that another frame is
    /// due.
    #[must_use]
    pub fn is_animating(&self, now: Instant) -> bool {
        self.is_fresh(now)
            || self.code_until.is_some_and(|until| now < until)
            || self.returning(now).is_some()
            || self
                .ghost
                .as_ref()
                .is_some_and(|ghost| ghost.is_rolling(now))
    }

    pub(crate) fn returning(&self, now: Instant) -> Option<(Range<usize>, f64)> {
        let returned = self.returned.as_ref()?;
        let since = now.saturating_duration_since(returned.at);
        let shown = match since.checked_sub(SETTLE.duration()) {
            None => 0.0,
            Some(fading) => SETTLE.eased(fading)?,
        };
        Some((returned.question.clone(), shown))
    }

    fn push(&mut self, entry: Entry, now: Instant) {
        self.settle_cold(now);
        let position = self.entries.len();
        if entry.last_arrival().is_some() {
            self.last_arrival = Some(now);
        }
        self.entries.push(entry);
        self.recount_from(position);
    }

    fn settle_cold(&mut self, now: Instant) {
        self.settled = self.settled.min(self.entries.len());
        while let Some(entry) = self.entries.get_mut(self.settled) {
            if !entry.settle(now) {
                return;
            }
            self.settled += 1;
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/history/mod.rs"]
mod tests;
