use std::ops::Range;

use crate::line::{TextWidth, WIDEST_COLUMN};
use crate::transcript::entry::{Entry, Kind};
use crate::wrap::wrap;

impl Entry {
    pub(crate) fn rewrap(&mut self, width: TextWidth) {
        if self.code().is_some() {
            self.recount_code();
            return;
        }
        if self.kind == Kind::Tool && self.text.is_empty() {
            return;
        }
        if let Some(summary) = self
            .mark
            .as_deref()
            .and_then(|mark| mark.summary.as_deref())
        {
            self.text.clear();
            summary.fit_into(width, &mut self.text);
        }
        self.rows = narrowed(wrap(&self.text, width.columns()), 0);
        self.rows.shrink_to_fit();
    }

    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn row(&self, index: usize) -> Range<usize> {
        widened(&self.rows[index])
    }

    pub(crate) fn shown_row(&self, index: usize) -> Range<usize> {
        let row = self.row(index);
        let end = self.text[row.clone()]
            .char_indices()
            .nth(usize::from(WIDEST_COLUMN))
            .map_or(row.end, |(at, _)| row.start + at);
        row.start..end
    }

    pub(crate) fn rows(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.rows.iter().map(widened)
    }

    pub(crate) fn row_holding(&self, offset: usize) -> usize {
        self.rows
            .partition_point(|row| widened(row).start <= offset)
            .saturating_sub(1)
    }

    pub(super) fn rewrap_tail(&mut self, width: TextWidth) {
        let start = self.rows.pop().map_or(0, |row| widened(&row).start);
        let tail = wrap(&self.text[start..], width.columns());
        self.rows.extend(narrowed(tail, start));
    }
}

pub(super) fn narrowed(rows: Vec<Range<usize>>, shift: usize) -> Vec<Range<u32>> {
    rows.into_iter()
        .map(|row| narrow(row.start + shift)..narrow(row.end + shift))
        .collect()
}

fn narrow(offset: usize) -> u32 {
    u32::try_from(offset).expect("an entry's text is kept under 4 GiB")
}

fn widened(row: &Range<u32>) -> Range<usize> {
    usize::try_from(row.start).unwrap_or(usize::MAX)..usize::try_from(row.end).unwrap_or(usize::MAX)
}
