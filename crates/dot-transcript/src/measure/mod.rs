mod calibration;
mod spark;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::measure::calibration::to_f64;
use crate::measure::spark::SAMPLE_EVERY;

pub use self::calibration::Calibration;
pub use self::spark::{SPARK_CELLS, SPARK_COLUMNS, Spark};

const RATE_WINDOW: Duration = Duration::from_secs(1);
const KEPT: Duration = Duration::from_secs(6);
const TICK: Duration = Duration::from_millis(100);
const SAMPLES_SHOWN: u32 = 20;

/// What one reply of Claude measured: what the instrument beside the reading line shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measures {
    /// Tokens Claude wrote: estimated from what it shows while it writes, exact once Claude
    /// counted them at the end of the reply; with those its agents wrote, counted as they come.
    pub tokens: u64,
    /// Tokens per second, over the last second while Claude writes, over all of its writing
    /// once the reply ended; `None` before anything was written.
    pub rate: Option<u64>,
    /// From the prompt sent to the end of the reply, or to now while it lasts.
    pub elapsed: Duration,
    /// The rate, a column per fifth of a second.
    pub spark: Spark,
}

/// The measures of a reply Claude writes, piece by piece.
#[derive(Clone, Debug, PartialEq)]
pub struct Meter {
    sent: Instant,
    first: Option<Instant>,
    ended: Option<Instant>,
    estimated: f64,
    exact: Option<u64>,
    characters: u64,
    counted: u64,
    arrivals: VecDeque<(Instant, f64)>,
}

impl Meter {
    /// Starts measuring the reply to a prompt sent at `sent`.
    #[must_use]
    pub const fn new(sent: Instant) -> Self {
        Self {
            sent,
            first: None,
            ended: None,
            estimated: 0.0,
            exact: None,
            characters: 0,
            counted: 0,
            arrivals: VecDeque::new(),
        }
    }

    /// Counts a piece of `characters` characters Claude showed at `now`, worth what
    /// `calibration` says; nothing once the reply ended.
    pub fn hear(&mut self, characters: usize, calibration: &Calibration, now: Instant) {
        if self.ended.is_some() || characters == 0 {
            return;
        }
        let tokens = calibration.tokens_for(characters);
        self.estimated += tokens;
        self.characters = self
            .characters
            .saturating_add(u64::try_from(characters).unwrap_or(u64::MAX));
        self.arrive(tokens, now);
    }

    /// Counts `tokens` that were counted already, such as those an agent Claude started wrote,
    /// at `now`; on top of the reply's own, and nothing once the reply ended.
    pub fn hear_tokens(&mut self, tokens: u64, now: Instant) {
        if self.ended.is_some() || tokens == 0 {
            return;
        }
        self.counted = self.counted.saturating_add(tokens);
        self.arrive(to_f64(tokens), now);
    }

    fn arrive(&mut self, tokens: f64, now: Instant) {
        self.first.get_or_insert(now);
        self.arrivals.push_back((now, tokens));
        while self
            .arrivals
            .front()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) > KEPT)
        {
            self.arrivals.pop_front();
        }
    }

    /// Ends the reply at `now`, with the tokens Claude counted for it when it did.
    pub fn end(&mut self, exact: Option<u64>, now: Instant) {
        if self.ended.is_some() {
            return;
        }
        self.ended = Some(now);
        self.exact = exact.filter(|&tokens| tokens > 0);
    }

    /// Returns whether Claude still writes the reply.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.ended.is_none()
    }

    /// Returns how many characters Claude showed, and the tokens it counted for the reply once
    /// it ended, if it did; those counted already ([`Meter::hear_tokens`]) are in neither.
    #[must_use]
    pub const fn counted(&self) -> (u64, Option<u64>) {
        (self.characters, self.exact)
    }

    /// Returns the measures at `now`.
    #[must_use]
    pub fn measures(&self, now: Instant) -> Measures {
        let until = self.ended.unwrap_or(now);
        let own = self.exact.unwrap_or_else(|| to_count(self.estimated));
        let tokens = own.saturating_add(self.counted);
        Measures {
            tokens,
            rate: self.rate(tokens, until),
            elapsed: until.saturating_duration_since(self.sent),
            spark: self.spark(until, now),
        }
    }

    /// Returns when the measures change next, while Claude writes: the next tenth of a second.
    #[must_use]
    pub fn next_change(&self, now: Instant) -> Option<Instant> {
        if self.ended.is_some() {
            return None;
        }
        let elapsed = now.saturating_duration_since(self.sent);
        let ticks = elapsed.as_millis() / TICK.as_millis() + 1;
        Some(self.sent + TICK * u32::try_from(ticks).unwrap_or(u32::MAX))
    }

    fn rate(&self, tokens: u64, until: Instant) -> Option<u64> {
        let first = self.first?;
        if self.ended.is_none() {
            return Some(to_count(self.written_over(until)));
        }
        let writing = until
            .saturating_duration_since(first)
            .max(Duration::from_millis(1));
        Some(to_count(to_f64(tokens) / writing.as_secs_f64()))
    }

    fn written_over(&self, until: Instant) -> f64 {
        self.arrivals
            .iter()
            .filter(|(at, _)| *at <= until && until.saturating_duration_since(*at) < RATE_WINDOW)
            .map(|(_, tokens)| tokens)
            .sum::<f64>()
            / RATE_WINDOW.as_secs_f64()
    }

    fn spark(&self, until: Instant, now: Instant) -> Spark {
        let Some(first) = self.first else {
            return Spark::default();
        };
        let samples = sample_count(until.saturating_duration_since(first));
        let shown = samples.min(SAMPLES_SHOWN);
        let rates: Vec<f64> = (samples - shown + 1..=samples)
            .map(|sample| self.written_over(first + SAMPLE_EVERY * sample))
            .collect();
        let newest = (self.ended.is_none() && samples > 0).then(|| first + SAMPLE_EVERY * samples);
        Spark::of(&rates, newest, now)
    }
}

fn sample_count(writing: Duration) -> u32 {
    let samples = writing.as_millis() / SAMPLE_EVERY.as_millis();
    u32::try_from(samples).unwrap_or(u32::MAX)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a count is rounded and floored at zero first, and a float cast to u64 saturates"
)]
fn to_count(value: f64) -> u64 {
    value.round().max(0.0) as u64
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/measure/mod.rs"]
mod tests;
