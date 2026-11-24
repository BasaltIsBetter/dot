mod aside;
mod rows;

use std::ops::Range;
use std::time::Instant;

use crate::code::CodeBlock;
use crate::line::TextWidth;
use crate::transcript::fade::is_fresh;
use crate::transcript::tool::{Mark, Tool};
use crate::wrap::{normalized, wrap};

use self::rows::narrowed;

const LONGEST_TEXT: usize = u32::MAX as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// What an entry of the conversation is.
pub enum Kind {
    /// What the user sent.
    Prompt,
    /// A note the user slipped into a reply Claude was writing, sent to Claude as a prompt of
    /// its own: the reply goes on below it.
    Note,
    /// An image sent with a prompt.
    Attachment,
    /// Claude's thinking, summarized.
    Thinking,
    /// Claude's answer.
    Answer,
    /// A notice: a reply that went wrong or was stopped, an image refused.
    Notice,
    /// A tool Claude runs: a mark in the gutter, and for a command once it ended, a line that
    /// sums it up.
    Tool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Arrival {
    pub(crate) start: usize,
    pub(crate) at: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) kind: Kind,
    pub(crate) text: String,
    rows: Vec<Range<u32>>,
    pub(crate) arrivals: Vec<Arrival>,
    pub(crate) ended: Option<Instant>,
    pub(crate) mark: Option<Box<Mark>>,
}

impl Entry {
    pub(crate) fn new(kind: Kind, text: &str, now: Instant, width: TextWidth) -> Self {
        let mut entry = Self::empty(kind, width);
        entry.extend(text, now, width);
        entry
    }

    pub(crate) fn empty(kind: Kind, width: TextWidth) -> Self {
        Self {
            kind,
            text: String::new(),
            rows: narrowed(wrap("", width.columns()), 0),
            arrivals: Vec::new(),
            ended: None,
            mark: None,
        }
    }

    pub(crate) fn tool(tool: Tool, now: Instant) -> Self {
        Self {
            kind: Kind::Tool,
            text: String::new(),
            rows: Vec::new(),
            arrivals: Vec::new(),
            ended: None,
            mark: Some(Box::new(Mark {
                tool,
                result: None,
                started: now,
                output: Vec::new(),
                code: None,
                summary: None,
            })),
        }
    }

    pub(crate) fn takes_rows(&self) -> bool {
        !self.rows.is_empty()
    }

    pub(crate) fn extend(&mut self, piece: &str, now: Instant, width: TextWidth) -> bool {
        let room = LONGEST_TEXT - self.text.len();
        let piece = normalized(piece, usize::MAX);
        let piece = &piece[..floor_char_boundary(&piece, room)];
        if piece.is_empty() {
            return false;
        }

        let settled = self
            .arrivals
            .partition_point(|arrival| !is_fresh(now.saturating_duration_since(arrival.at)));
        self.arrivals.drain(..settled);
        self.arrivals.push(Arrival {
            start: self.text.len(),
            at: now,
        });
        self.text.push_str(piece);
        self.rewrap_tail(width);
        true
    }

    pub(crate) fn give_back_room(&mut self) {
        self.text.shrink_to_fit();
        self.rows.shrink_to_fit();
    }

    pub(crate) fn settle(&mut self, now: Instant) -> bool {
        let fresh = |at: Instant| is_fresh(now.saturating_duration_since(at));
        let replaced = self.replaced().map(|replaced| replaced.at);
        if self.last_arrival().is_some_and(fresh) || replaced.is_some_and(fresh) {
            return false;
        }
        self.arrivals = Vec::new();
        self.text.shrink_to_fit();
        self.rows.shrink_to_fit();
        if let Some(summary) = self
            .mark
            .as_deref_mut()
            .and_then(|mark| mark.summary.as_deref_mut())
        {
            summary.replaced = None;
        }
        true
    }

    pub(crate) fn end(&mut self, now: Instant) {
        self.ended = Some(now);
        if let Some(mark) = self.mark.as_mut() {
            mark.output = Vec::new();
        }
    }

    pub(crate) fn show_code(&mut self, code: CodeBlock) {
        if let Some(mark) = self.mark.as_deref_mut() {
            mark.code = Some(Box::new(code));
            self.recount_code();
        }
    }

    pub(crate) fn code(&self) -> Option<&CodeBlock> {
        self.mark.as_deref()?.code.as_deref()
    }

    pub(crate) fn code_mut(&mut self) -> Option<&mut CodeBlock> {
        self.mark.as_deref_mut()?.code.as_deref_mut()
    }

    pub(crate) fn recount_code(&mut self) {
        let rows = self.code().map_or(0, CodeBlock::row_count);
        self.rows = vec![0..0; rows];
    }
}

fn floor_char_boundary(text: &str, at: usize) -> usize {
    if at >= text.len() {
        return text.len();
    }
    (0..=at)
        .rev()
        .find(|&index| text.is_char_boundary(index))
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/entry/mod.rs"]
mod tests;
