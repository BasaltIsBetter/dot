use std::ops::Range;
use std::time::Instant;

use crate::canvas::{SETTLE, columns};
use crate::conversation::transcript::entry::Entry;

const SPACE: char = ' ';
const PAST_EVERY_WORD: Cell = (usize::MAX, usize::MAX);

pub(crate) type Cell = (usize, usize);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Ghost {
    question: Range<usize>,
    last_word: Cell,
    advances: Vec<Advance>,
    rolls_until: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Advance {
    cursor: Cell,
    at: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Fate {
    Waiting,
    Leaving(f64),
    Gone,
}

impl Ghost {
    pub(crate) fn new(question: Range<usize>, entries: &[Entry]) -> Self {
        Self {
            last_word: last_word(&entries[question.clone()]),
            question,
            advances: Vec::new(),
            rolls_until: None,
        }
    }

    pub(crate) fn question(&self) -> Range<usize> {
        self.question.clone()
    }

    pub(crate) const fn slot(&self) -> usize {
        self.question.end
    }

    pub(crate) const fn is_complete(&self) -> bool {
        self.rolls_until.is_some()
    }

    pub(crate) fn hides(&self, position: usize) -> bool {
        !self.is_complete() && self.question.contains(&position)
    }

    pub(crate) fn advance(&mut self, cursor: Cell, written: usize, now: Instant) {
        if self.is_complete() {
            return;
        }
        self.advances.push(Advance {
            cursor,
            at: Some(now),
        });
        if cursor > self.last_word {
            self.rolls_until = Some(written);
        }
    }

    pub(crate) fn finish(&mut self, written: usize, now: Instant) {
        self.advance(PAST_EVERY_WORD, written, now);
    }

    pub(crate) fn rebase(&mut self, entries: &[Entry], cursor: Cell, written: usize) {
        self.last_word = last_word(&entries[self.question.clone()]);
        let reached = if self.is_complete() {
            PAST_EVERY_WORD
        } else {
            cursor
        };
        self.advances = vec![Advance {
            cursor: reached,
            at: None,
        }];
        if reached > self.last_word {
            self.rolls_until.get_or_insert(written);
        }
    }

    pub(crate) fn fate(&self, cell: Cell, now: Instant) -> Fate {
        let reaching = self
            .advances
            .partition_point(|advance| advance.cursor <= cell);
        let Some(advance) = self.advances.get(reaching) else {
            return Fate::Waiting;
        };
        advance
            .at
            .and_then(|at| SETTLE.eased(now.saturating_duration_since(at)))
            .map_or(Fate::Gone, Fate::Leaving)
    }

    pub(crate) fn rolls(&self, start: usize) -> bool {
        self.rolls_until.is_none_or(|until| start < until)
    }

    pub(crate) fn is_rolling(&self, now: Instant) -> bool {
        self.advances
            .last()
            .and_then(|advance| advance.at)
            .is_some_and(|at| SETTLE.lasts(now.saturating_duration_since(at)))
    }

    pub(crate) fn is_settled(&self, now: Instant) -> bool {
        self.is_complete() && !self.is_rolling(now)
    }
}

pub(crate) fn words(text: &str) -> impl Iterator<Item = (usize, Range<usize>)> + '_ {
    let mut column = 0;
    let mut byte = 0;
    text.split(SPACE).filter_map(move |word| {
        let (start, at) = (byte, column);
        byte += word.len() + SPACE.len_utf8();
        column += columns(word) + 1;
        (!word.is_empty()).then_some((at, start..start + word.len()))
    })
}

fn last_word(question: &[Entry]) -> Cell {
    let rows = question
        .iter()
        .flat_map(|entry| entry.rows().map(move |row| &entry.text[row]));
    rows.enumerate()
        .filter_map(|(row, text)| words(text).last().map(|(column, _)| (row, column)))
        .last()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/ghost.rs"]
mod tests;
