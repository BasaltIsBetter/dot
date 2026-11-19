use std::ops::Range;
use std::time::Instant;

use crate::conversation::transcript::fade::{HOTTEST, heat, is_fresh};
use crate::conversation::wrap::{normalized, rewrap_tail, wrap};
use crate::conversation::zone::TextWidth;

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
    pub(crate) rows: Vec<Range<usize>>,
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
            rows: wrap("", width.columns()),
            arrivals: Vec::new(),
            ended: None,
        }
    }

    pub(crate) fn extend(&mut self, piece: &str, now: Instant, width: TextWidth) {
        let piece = normalized(piece, usize::MAX);
        if piece.is_empty() {
            return;
        }

        let settled = self
            .arrivals
            .partition_point(|arrival| !is_fresh(now.saturating_duration_since(arrival.at)));
        self.arrivals.drain(..settled);
        self.arrivals.push(Arrival {
            start: self.text.len(),
            at: now,
        });
        self.text.push_str(&piece);
        rewrap_tail(&self.text, &mut self.rows, width.columns());
    }

    pub(crate) fn rewrap(&mut self, width: TextWidth) {
        self.rows = wrap(&self.text, width.columns());
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
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
        let Some(last) = self.rows.last() else {
            return (0, 0);
        };
        let column = self.text[last.start..last.end].chars().count();
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

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/entry.rs"]
mod tests;
