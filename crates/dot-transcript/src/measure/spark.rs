use std::time::{Duration, Instant};

use crate::transcript::heat_after;

/// Columns of a spark: one per fifth of a second, the last four seconds.
pub const SPARK_COLUMNS: usize = 20;
/// Cells of a column of a spark, lit from the bottom.
pub const SPARK_CELLS: u8 = 5;

pub(crate) const SAMPLE_EVERY: Duration = Duration::from_millis(200);

const TOKENS_PER_SECOND_PER_CELL: f64 = 18.0;
const FLASH: Duration = Duration::from_millis(120);
const FLASH_COOLING: Duration = Duration::from_millis(450);
const HOTTEST: u8 = u8::MAX;

/// The rate of a reply drawn as a row of small columns, the newest on the right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spark {
    /// Lit cells of each column, oldest first, from 0 to [`SPARK_CELLS`].
    pub cells: [u8; SPARK_COLUMNS],
    /// Tokens per second of each column, oldest first, rounded.
    pub rates: [u16; SPARK_COLUMNS],
    /// How much of the live colour the lit cells of the newest column still have: they flash
    /// when the column is sampled, while Claude writes, then cool.
    pub flash: u8,
}

impl Spark {
    pub(crate) fn of(rates: &[f64], sampled: Option<Instant>, now: Instant) -> Self {
        let mut cells = [0; SPARK_COLUMNS];
        let mut rounded = [0; SPARK_COLUMNS];
        let shown = &rates[rates.len().saturating_sub(SPARK_COLUMNS)..];
        let empty = SPARK_COLUMNS - shown.len();
        for (column, rate) in shown.iter().enumerate() {
            cells[empty + column] = cells_for(*rate);
            rounded[empty + column] = to_rate(*rate);
        }
        let flash = sampled.map_or(0, |sampled| {
            flash_after(now.saturating_duration_since(sampled))
        });
        Self {
            cells,
            rates: rounded,
            flash,
        }
    }

    /// Returns whether the newest column is still cooling from its flash.
    #[must_use]
    pub fn is_flashing(&self) -> bool {
        self.flash > 0 && self.cells[SPARK_COLUMNS - 1] > 0
    }
}

pub(crate) fn cells_for(rate: f64) -> u8 {
    if rate.is_nan() || rate <= 0.0 {
        return 0;
    }
    let cells = (rate / TOKENS_PER_SECOND_PER_CELL).round();
    if cells >= f64::from(SPARK_CELLS) {
        return SPARK_CELLS;
    }
    to_cells(cells).max(1)
}

fn flash_after(elapsed: Duration) -> u8 {
    match elapsed.checked_sub(FLASH) {
        None => HOTTEST,
        Some(cooling) => heat_after(cooling, FLASH_COOLING),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the count is rounded and checked below the cells of a column before the cast"
)]
const fn to_cells(cells: f64) -> u8 {
    cells as u8
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a rate is clamped to the range of u16 before the cast, NaN to zero"
)]
fn to_rate(rate: f64) -> u16 {
    if rate.is_nan() {
        return 0;
    }
    rate.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/measure/spark.rs"]
mod tests;
