use std::ops::Range;
use std::time::{Duration, Instant};

use crate::motion::ease_out;
use crate::transcript::entry::{Arrival, Entry, Kind};

const FRESH: Duration = Duration::from_millis(1_200);
const HOTTEST_SHARE: f64 = 255.0;

const HOTTEST: u8 = u8::MAX;

pub(crate) fn is_fresh(elapsed: Duration) -> bool {
    elapsed < FRESH
}

pub(crate) fn heat(elapsed: Duration) -> u8 {
    heat_after(elapsed, FRESH)
}

/// Returns how orange something still is `elapsed` after it arrived, when it cools to its ink
/// along CSS's `ease-out` curve over `cooling`: all of it at first, none once `cooling` is over.
#[must_use]
pub fn heat_after(elapsed: Duration, cooling: Duration) -> u8 {
    if elapsed >= cooling {
        return 0;
    }
    let progress = elapsed.as_secs_f64() / cooling.as_secs_f64();
    to_heat(1.0 - ease_out(progress))
}

impl Entry {
    pub(crate) fn heat(&self, arrival: Option<&Arrival>, now: Instant) -> u8 {
        match self.kind {
            Kind::Tool => self
                .ended
                .map_or(HOTTEST, |ended| heat(now.saturating_duration_since(ended))),
            Kind::Prompt | Kind::Attachment => 0,
            Kind::Note | Kind::Thinking | Kind::Answer | Kind::Notice => {
                arrival.map_or(0, |arrival| heat(now.saturating_duration_since(arrival.at)))
            }
        }
    }

    pub(crate) fn last_arrival(&self) -> Option<Instant> {
        self.arrivals.last().map(|arrival| arrival.at)
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

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the share is clamped to [0, 1] before scaling to the range of u8"
)]
fn to_heat(share: f64) -> u8 {
    (share.clamp(0.0, 1.0) * HOTTEST_SHARE).round() as u8
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/transcript/fade.rs"]
mod tests;
