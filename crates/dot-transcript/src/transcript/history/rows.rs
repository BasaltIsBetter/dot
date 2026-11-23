use std::time::Instant;

use crate::line::TextWidth;
use crate::reading::{index, rows};
use crate::transcript::entry::{Entry, Kind};
use crate::transcript::history::{Row, Transcript};

const LOOKED_AROUND: [isize; 5] = [0, -1, 1, -2, 2];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Anchor {
    entry: usize,
    offset: Option<usize>,
    fraction: f64,
}

impl Transcript {
    /// Wraps every entry again at `width` at `now`, and returns the row that now holds what row
    /// `focus` held, so that whatever is read keeps its place.
    pub fn rewrap(&mut self, width: TextWidth, focus: f64, now: Instant) -> f64 {
        let total_before = self.total();
        let anchor = self.anchor_at(focus);
        self.width = width;
        for entry in &mut self.entries {
            entry.rewrap(width);
        }
        self.recount_from(0);
        self.rebase_ghost(now);
        self.insertions.clear();

        match anchor {
            Some(anchor) => self.row_of(anchor),
            None => rows(self.total()) + (focus - rows(total_before)),
        }
    }

    /// Returns the exchange row `row` belongs to, counted from 0 at the first prompt: that of
    /// the nearest row with text, the row itself first, then up to two rows away; `None` above
    /// the first prompt or far from any text.
    #[must_use]
    pub fn exchange_at(&self, row: usize) -> Option<usize> {
        let position = LOOKED_AROUND.iter().find_map(|&offset| {
            match self.row(row.checked_add_signed(offset)?)? {
                Row::Text { position, .. } => Some(position),
                Row::Gap => None,
            }
        })?;
        self.prompts
            .partition_point(|&prompt| prompt <= position)
            .checked_sub(1)
    }

    /// Returns how many exchanges the transcript holds: one per prompt sent.
    #[must_use]
    pub fn exchanges(&self) -> usize {
        self.prompts.len()
    }

    /// Returns how many rows the entries take.
    #[must_use]
    pub fn total(&self) -> usize {
        self.ends.last().copied().unwrap_or_default()
    }

    /// Returns the row past the last one shown: past the entries, or past the question while it
    /// waits to be replaced, if it reaches further.
    #[must_use]
    pub fn end_row(&self) -> usize {
        let total = self.total();
        let waiting_question = self
            .ghost
            .as_ref()
            .filter(|ghost| !ghost.is_complete())
            .and(self.anchor());
        waiting_question.map_or(total, |anchor| total.max(anchor + self.ghost_rows()))
    }

    /// Returns the first row of the latest exchange, from the last prompt on: what is read in
    /// full ink. Without any prompt, the row past the entries.
    #[must_use]
    pub fn active_from(&self) -> usize {
        self.last_prompt
            .and_then(|prompt| self.first_row_of(prompt))
            .unwrap_or_else(|| self.total())
    }

    pub(crate) fn row(&self, index: usize) -> Option<Row<'_>> {
        let position = self.ends.partition_point(|&end| end <= index);
        let entry = self.entries.get(position)?;
        let local = index - self.start_of(position);
        Some(
            local
                .checked_sub(self.gap_at(position))
                .map_or(Row::Gap, |row| Row::Text {
                    entry,
                    row,
                    position,
                }),
        )
    }

    pub(crate) fn first_row_of(&self, position: usize) -> Option<usize> {
        self.entries.get(position)?;
        Some(self.start_of(position) + self.gap_at(position))
    }

    pub(crate) fn is_last_row(&self, index: usize) -> bool {
        index + 1 == self.total()
    }

    pub(crate) fn takes_rows(&self, position: usize) -> bool {
        self.ends
            .get(position)
            .is_some_and(|&end| end > self.start_of(position))
    }

    pub(super) fn recount_from(&mut self, from: usize) {
        let from = from.min(self.entries.len());
        self.ends.truncate(from);
        self.gaps.truncate(from);
        let mut end = self.total();
        let mut previous = self.visible_before(from);
        for position in from..self.entries.len() {
            let entry = &self.entries[position];
            let shown = !self.is_hidden(position) && entry.takes_rows();
            let gap = shown
                && previous.is_some_and(|previous| gap_between(&self.entries[previous], entry));
            if shown {
                end += usize::from(gap) + entry.row_count();
                previous = Some(position);
            }
            self.ends.push(end);
            self.gaps.push(gap);
        }
    }

    pub(super) fn start_of(&self, position: usize) -> usize {
        position
            .checked_sub(1)
            .and_then(|previous| self.ends.get(previous))
            .copied()
            .unwrap_or_default()
    }

    pub(super) fn gap_at(&self, position: usize) -> usize {
        usize::from(self.gaps.get(position).copied().unwrap_or_default())
    }

    fn is_hidden(&self, position: usize) -> bool {
        self.ghost
            .as_ref()
            .is_some_and(|ghost| ghost.hides(position))
    }

    fn visible_before(&self, position: usize) -> Option<usize> {
        (0..position)
            .rev()
            .find(|&earlier| self.takes_rows(earlier))
    }

    fn anchor_at(&self, first: f64) -> Option<Anchor> {
        let top = first.floor();
        let top_row = index(top);
        let position = self.ends.partition_point(|&end| end <= top_row);
        let entry = self.entries.get(position)?;
        let row = (top_row - self.start_of(position)).checked_sub(self.gap_at(position));
        Some(Anchor {
            entry: position,
            offset: row.map(|row| entry.row(row).start),
            fraction: first - top,
        })
    }

    fn row_of(&self, anchor: Anchor) -> f64 {
        let entry: &Entry = &self.entries[anchor.entry];
        let row = anchor.offset.map_or(0, |offset| {
            self.gap_at(anchor.entry) + entry.row_holding(offset)
        });
        rows(self.start_of(anchor.entry) + row) + anchor.fraction
    }
}

fn gap_between(previous: &Entry, entry: &Entry) -> bool {
    let asides_in_a_row = previous.kind == Kind::Tool
        && entry.kind == Kind::Tool
        && previous.code().is_none()
        && entry.code().is_none();
    entry.kind != Kind::Attachment && !asides_in_a_row
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/history/rows.rs"]
mod tests;
