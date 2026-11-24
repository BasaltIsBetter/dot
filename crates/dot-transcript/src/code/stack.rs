use std::time::Instant;

use crate::code::{Body, ChangeHunk, ChangeLine, CodeBlock};
use crate::ink::Ink;
use crate::line::{Line, Span, TEXT_INDENT, columns, truncated};

const CREATED: &str = "✚ ";
const OPENS: &str = " ↗";
const BEFORE_THE_TALLY: &str = "   ";
const FOLDED: &str = "··· ";
const MINUS: char = '−';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    added: u64,
    removed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stacked {
    folded: bool,
    pad: usize,
}

struct Header<'a> {
    name: &'a str,
    created: bool,
    tally: Tally,
    pad: usize,
    room: usize,
}

impl Tally {
    pub(crate) fn of(hunks: &[ChangeHunk]) -> Self {
        hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .fold(Self::default(), |tally, line| match line {
                ChangeLine::Added(_) => Self {
                    added: tally.added.saturating_add(1),
                    ..tally
                },
                ChangeLine::Removed(_) => Self {
                    removed: tally.removed.saturating_add(1),
                    ..tally
                },
                ChangeLine::Same(_) => tally,
            })
    }

    fn text(self, created: bool) -> String {
        if created {
            format!("+{}", self.added)
        } else {
            format!("+{} {MINUS}{}", self.added, self.removed)
        }
    }
}

impl Header<'_> {
    fn line(&self) -> Line {
        let tally = format!(
            "{BEFORE_THE_TALLY}{FOLDED}{}",
            self.tally.text(self.created)
        );
        let own = name_columns(self.name, self.created);
        let padding = self.pad.saturating_sub(own);
        let room = self
            .room
            .saturating_sub(
                usize::from(TEXT_INDENT) + columns(prefix(self.created)) + columns(OPENS),
            )
            .saturating_sub(padding + columns(&tally));
        let name = format!("{}{}", prefix(self.created), truncated(self.name, room));
        Line::new(
            TEXT_INDENT,
            vec![
                Span::new(name, Ink::Secondary),
                Span::new(format!("{OPENS}{}", " ".repeat(padding)), Ink::Secondary),
                Span::new(tally, Ink::Receding),
            ],
        )
    }
}

impl CodeBlock {
    pub(crate) fn header_columns(&self) -> usize {
        name_columns(self.name(), self.is_created())
    }

    pub(crate) fn stack(&mut self, pad: usize) {
        match self.stacked.as_mut() {
            Some(stacked) => stacked.pad = pad,
            None => self.stacked = Some(Stacked { folded: true, pad }),
        }
    }

    pub(super) fn is_folded(&self) -> bool {
        self.stacked.is_some_and(|stacked| stacked.folded)
    }

    pub(super) fn shift(&self) -> usize {
        usize::from(self.stacked.is_some() && self.is_created())
    }

    pub(super) fn body_row(&self, row: usize) -> Option<usize> {
        if self.is_folded() {
            return None;
        }
        row.checked_sub(self.shift())
    }

    pub(super) const fn heads_a_stack(&self, row: usize) -> bool {
        self.stacked.is_some() && row == 0
    }

    pub(super) fn toggle_stack(&mut self, column: usize) -> bool {
        let end = name_end(self.name(), self.is_created());
        match self.stacked.as_mut().filter(|_| column >= end) {
            Some(stacked) => {
                stacked.folded = !stacked.folded;
                true
            }
            None => false,
        }
    }

    pub(super) fn names_the_file(&self, column: usize, now: Instant) -> bool {
        let written = match &self.body {
            Body::Edit(_) => true,
            Body::Create(created) => created.is_written(self.elapsed(now)),
        };
        written && column < name_end(self.name(), self.is_created())
    }

    pub(super) fn header(&self, room: usize) -> Line {
        Header {
            name: self.name(),
            created: self.is_created(),
            tally: self.tally,
            pad: self.stacked.map_or(0, |stacked| stacked.pad),
            room,
        }
        .line()
    }

    fn is_created(&self) -> bool {
        matches!(self.body, Body::Create(_))
    }
}

pub(crate) fn name_columns(name: &str, created: bool) -> usize {
    columns(prefix(created)) + columns(name) + columns(OPENS)
}

pub(crate) fn name_end(name: &str, created: bool) -> usize {
    usize::from(TEXT_INDENT) + name_columns(name, created)
}

const fn prefix(created: bool) -> &'static str {
    if created { CREATED } else { "" }
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/code/stack.rs"]
mod tests;
