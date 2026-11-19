use std::ops::Range;
use std::time::Instant;

use crate::canvas::columns;
use crate::conversation::transcript::fade::{HOTTEST, heat, is_fresh};
use crate::conversation::wrap::{normalized, wrap};
use crate::conversation::zone::TextWidth;

const LONGEST_TEXT: usize = u32::MAX as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Prompt,
    Attachment,
    Thinking,
    Answer,
    Notice,
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
        }
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

    pub(crate) fn rewrap(&mut self, width: TextWidth) {
        self.rows = narrowed(wrap(&self.text, width.columns()), 0);
        self.rows.shrink_to_fit();
    }

    pub(crate) fn settle(&mut self, now: Instant) -> bool {
        let hot = self
            .last_arrival()
            .is_some_and(|arrival| is_fresh(now.saturating_duration_since(arrival)));
        if hot {
            return false;
        }
        self.arrivals = Vec::new();
        self.text.shrink_to_fit();
        self.rows.shrink_to_fit();
        true
    }

    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn row(&self, index: usize) -> Range<usize> {
        widened(&self.rows[index])
    }

    pub(crate) fn rows(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.rows.iter().map(widened)
    }

    pub(crate) fn row_holding(&self, offset: usize) -> usize {
        self.rows
            .partition_point(|row| widened(row).start <= offset)
            .saturating_sub(1)
    }

    fn rewrap_tail(&mut self, width: TextWidth) {
        let start = self.rows.pop().map_or(0, |row| widened(&row).start);
        let tail = wrap(&self.text[start..], width.columns());
        self.rows.extend(narrowed(tail, start));
    }

    pub(crate) fn heat(&self, arrival: Option<&Arrival>, now: Instant) -> u8 {
        if self.kind != Kind::Tool {
            return arrival.map_or(0, |arrival| heat(now.saturating_duration_since(arrival.at)));
        }
        self.ended
            .map_or(HOTTEST, |ended| heat(now.saturating_duration_since(ended)))
    }

    pub(crate) fn last_arrival(&self) -> Option<Instant> {
        self.arrivals.last().map(|arrival| arrival.at)
    }

    pub(crate) fn cursor(&self) -> (usize, usize) {
        let Some(last) = self.rows.last().map(widened) else {
            return (0, 0);
        };
        let column = columns(&self.text[last]);
        (self.rows.len() - 1, column)
    }

    pub(crate) fn each_piece(
        &self,
        row: Range<usize>,
        now: Instant,
        piece: impl FnMut(Range<usize>, u8),
    ) {
        self.each_piece_by(row, |arrival| self.heat(arrival, now), piece);
    }

    pub(crate) fn each_piece_by<K: PartialEq>(
        &self,
        row: Range<usize>,
        key_of: impl Fn(Option<&Arrival>) -> K,
        mut piece: impl FnMut(Range<usize>, K),
    ) {
        let following = self
            .arrivals
            .partition_point(|arrival| arrival.start <= row.start);
        let mut current = key_of(
            following
                .checked_sub(1)
                .map(|earlier| &self.arrivals[earlier]),
        );
        let mut start = row.start;

        for arrival in &self.arrivals[following..] {
            if arrival.start >= row.end {
                break;
            }
            let arriving = key_of(Some(arrival));
            if arriving != current {
                piece(start..arrival.start, current);
                start = arrival.start;
                current = arriving;
            }
        }
        piece(start..row.end, current);
    }
}

fn narrowed(rows: Vec<Range<usize>>, shift: usize) -> Vec<Range<u32>> {
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
#[path = "../../../../../tests/dot-desktop/conversation/transcript/entry.rs"]
mod tests;
