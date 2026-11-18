use std::ops::Range;
use std::time::Instant;

use crate::conversation::TEXT_WIDTH;
use crate::conversation::scroll::{Scroll, Window};
use crate::conversation::transcript::fade::is_fresh;
use crate::conversation::wrap::wrap;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
    entries: Vec<Entry>,
    ends: Vec<usize>,
    scroll: Scroll,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) text: String,
    pub(crate) rows: Vec<Range<usize>>,
    pub(crate) arrived_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row<'a> {
    Gap,
    Text { entry: &'a Entry, row: usize },
}

impl Transcript {
    pub(crate) fn append(&mut self, text: String, now: Instant) {
        let rows = wrap(&text, TEXT_WIDTH);
        let gap = usize::from(!self.entries.is_empty());
        let end = self.total() + gap + rows.len();

        self.entries.push(Entry {
            text,
            rows,
            arrived_at: now,
        });
        self.ends.push(end);
        self.scroll.follow();
    }

    pub(crate) fn total(&self) -> usize {
        self.ends.last().copied().unwrap_or_default()
    }

    pub(crate) fn scroll_by(&mut self, rows: f64, visible: f64) {
        self.scroll.by(rows, self.total(), visible);
    }

    pub(crate) fn refit(&mut self, visible: f64) {
        self.scroll.refit(self.total(), visible);
    }

    pub(crate) fn window(&self, visible: f64) -> Window {
        self.scroll.window(self.total(), visible)
    }

    pub(crate) fn row(&self, index: usize) -> Option<Row<'_>> {
        let position = self.ends.partition_point(|&end| end <= index);
        let entry = self.entries.get(position)?;
        let begin = position
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);
        let local = index - begin;
        if position == 0 {
            return Some(Row::Text { entry, row: local });
        }
        Some(
            local
                .checked_sub(1)
                .map_or(Row::Gap, |row| Row::Text { entry, row }),
        )
    }

    pub(crate) fn is_fresh(&self, now: Instant) -> bool {
        self.entries
            .last()
            .is_some_and(|entry| is_fresh(now.saturating_duration_since(entry.arrived_at)))
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/history.rs"]
mod tests;
