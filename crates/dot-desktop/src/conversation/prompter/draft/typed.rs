use std::ops::Range;
use std::time::{Duration, Instant};

use dot_transcript::heat_after;

const COOLING: Duration = Duration::from_millis(450);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Typed {
    fresh: Vec<Arrival>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Arrival {
    bytes: Range<usize>,
    at: Instant,
}

impl Typed {
    pub(crate) fn inserted(&mut self, at: usize, length: usize, now: Instant) {
        self.forget_cold(now);
        for arrival in &mut self.fresh {
            if arrival.bytes.start >= at {
                arrival.bytes.start += length;
                arrival.bytes.end += length;
            } else if arrival.bytes.end > at {
                arrival.bytes.end += length;
            }
        }
        self.fresh.push(Arrival {
            bytes: at..at + length,
            at: now,
        });
    }

    pub(crate) fn erased(&mut self, erased: &Range<usize>, now: Instant) {
        self.forget_cold(now);
        let length = erased.len();
        let moved = |offset: usize| {
            if offset >= erased.end {
                offset - length
            } else {
                offset.min(erased.start)
            }
        };
        for arrival in &mut self.fresh {
            arrival.bytes = moved(arrival.bytes.start)..moved(arrival.bytes.end);
        }
        self.fresh.retain(|arrival| !arrival.bytes.is_empty());
    }

    pub(crate) fn heat_at(&self, byte: usize, now: Instant) -> u8 {
        self.fresh
            .iter()
            .rev()
            .find(|arrival| arrival.bytes.contains(&byte))
            .map_or(0, |arrival| {
                heat_after(now.saturating_duration_since(arrival.at), COOLING)
            })
    }

    pub(crate) fn runs(&self, row: Range<usize>, now: Instant) -> Vec<(Range<usize>, u8)> {
        let mut edges: Vec<usize> = self
            .fresh
            .iter()
            .flat_map(|arrival| [arrival.bytes.start, arrival.bytes.end])
            .filter(|edge| row.contains(edge))
            .chain([row.start, row.end])
            .collect();
        edges.sort_unstable();
        edges.dedup();

        let mut runs: Vec<(Range<usize>, u8)> = Vec::new();
        for pair in edges.windows(2) {
            let piece = pair[0]..pair[1];
            let heat = self.heat_at(piece.start, now);
            match runs.last_mut() {
                Some((last, last_heat)) if *last_heat == heat => last.end = piece.end,
                _ => runs.push((piece, heat)),
            }
        }
        runs
    }

    pub(crate) fn is_cooling(&self, now: Instant) -> bool {
        self.fresh
            .iter()
            .any(|arrival| now.saturating_duration_since(arrival.at) < COOLING)
    }

    fn forget_cold(&mut self, now: Instant) {
        self.fresh
            .retain(|arrival| now.saturating_duration_since(arrival.at) < COOLING);
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/prompter/draft/typed.rs"]
mod tests;
