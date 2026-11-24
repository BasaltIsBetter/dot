mod cadence;
mod window;

use std::time::Instant;

pub use self::cadence::{Cadence, words_within};
pub(crate) use self::window::Window;
pub use self::window::{index, rows, to_f32};
use crate::motion::{Glide, SETTLE};

const REVIEWING_FROM: f64 = 0.5;
const CENTRED_WITHIN: f64 = 0.5;

/// Where a teleprompter reads: the row on its fixed reading line, gliding to its place, and how
/// far the reader has scrolled away from where the conversation rests.
///
/// The conversation tells it, frame after frame, the row it rests on (the row being typed, or
/// where Claude's reply starts) and the row Claude writes on; the reader keeps the row on the
/// line still while the reader looks back, and comes back to the resting row otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reader {
    offset: f64,
    resting: Option<f64>,
    target: Option<f64>,
    lag: Option<Glide>,
    band_target: Option<f64>,
    band_lag: Option<Glide>,
    landing: Option<f64>,
}

/// Where the conversation stands for one frame, in rows of the whole flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The row the reading line rests on while nobody scrolls.
    pub resting: usize,
    /// The last row of the flow.
    pub last: usize,
    /// The row Claude writes on, if Claude writes.
    pub writing: Option<usize>,
}

impl Reader {
    /// Settles the reader on `plan` at `now`: a reader looking back keeps the same row on the
    /// line whatever the conversation does; any other glides to the resting row.
    pub fn settle(&mut self, plan: Plan, now: Instant) {
        let resting = rows(plan.resting);
        let landing = self.landing.take();
        match (landing, self.resting) {
            (Some(row), _) if self.is_reviewing() => self.offset = row - resting,
            (None, Some(previous)) if self.is_reviewing() => self.offset -= resting - previous,
            _ => {}
        }
        self.resting = Some(resting);
        let room_below = (rows(plan.last) - resting).max(0.0);
        self.offset = self.offset.clamp(-resting, room_below);

        let target = resting + self.offset.round();
        let band = match plan.writing {
            Some(writing) if !self.is_reviewing() => rows(writing),
            _ => target,
        };
        if landing.is_some() {
            self.target = Some(target);
            self.lag = None;
            self.band_target = Some(band);
            self.band_lag = None;
            return;
        }
        aim(&mut self.target, &mut self.lag, target, now);
        aim(&mut self.band_target, &mut self.band_lag, band, now);
    }

    /// Scrolls by `rows` toward the end of the flow (toward its start when negative).
    pub fn scroll_by(&mut self, rows: f64) {
        if rows.is_finite() {
            self.offset += rows;
        }
    }

    /// Comes back to the resting row: the reader stops looking back.
    pub const fn come_back(&mut self) {
        self.offset = 0.0;
    }

    /// Follows `rows` rows inserted at row `at`: whatever was at or past `at` moved down by that
    /// much, so the reader moves along with it when it looks there, and nothing on screen moves.
    pub fn shift(&mut self, at: f64, rows: f64) {
        let moved = |row: f64| if row >= at { row + rows } else { row };
        if let (Some(target), Some(resting)) = (self.target, self.resting) {
            self.offset += (moved(target) - target) - (moved(resting) - resting);
        }
        self.target = self.target.map(moved);
        self.resting = self.resting.map(moved);
        self.band_target = self.band_target.map(moved);
    }

    /// Makes the next settle land at once, without gliding: once the flow wrapped again, a
    /// reader looking back keeps `row` on the line.
    pub const fn land_on(&mut self, row: f64) {
        self.landing = Some(row);
    }

    /// Returns the row shown on the reading line at `now`, part of a row while it glides.
    #[must_use]
    pub fn focus(&self, now: Instant) -> f64 {
        shown(self.target, self.lag, now)
    }

    /// Returns how many rows the reading band is from the reading line at `now`: below it when
    /// positive, while the band follows Claude's writing.
    #[must_use]
    pub fn band(&self, now: Instant) -> f64 {
        shown(self.band_target, self.band_lag, now) - self.focus(now)
    }

    /// Returns whether the reader has scrolled away from the resting row.
    #[must_use]
    pub fn is_reviewing(&self) -> bool {
        self.offset.abs() >= REVIEWING_FROM
    }

    /// Returns whether the reader rests on the resting row at `now`, and is nearly there.
    #[must_use]
    pub fn is_centred(&self, now: Instant) -> bool {
        !self.is_reviewing() && lag_at(self.lag, now).abs() < CENTRED_WITHIN
    }

    /// Returns whether the line or the band still glides at `now`.
    #[must_use]
    pub fn is_moving(&self, now: Instant) -> bool {
        [self.lag, self.band_lag]
            .into_iter()
            .flatten()
            .any(|glide| glide.is_moving(now))
    }
}

fn aim(target: &mut Option<f64>, lag: &mut Option<Glide>, aimed: f64, now: Instant) {
    let Some(previous) = *target else {
        *target = Some(aimed);
        return;
    };
    if (previous - aimed).abs() < f64::EPSILON {
        return;
    }
    let departure = previous + lag_at(*lag, now) - aimed;
    let mut glide = Glide::at_rest(to_f32(departure), SETTLE);
    glide.toward(0.0, now);
    *target = Some(aimed);
    *lag = Some(glide);
}

fn shown(target: Option<f64>, lag: Option<Glide>, now: Instant) -> f64 {
    target.unwrap_or_default() + lag_at(lag, now)
}

fn lag_at(lag: Option<Glide>, now: Instant) -> f64 {
    lag.map_or(0.0, |glide| f64::from(glide.position(now)))
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/reading/mod.rs"]
mod tests;
