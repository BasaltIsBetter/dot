use std::time::{Duration, Instant};

use crate::line::columns;

const SLOWEST: f64 = 80.0;
const CATCH_UP: f64 = 0.5;
const LONGEST_STEP: Duration = Duration::from_millis(100);
const LONGEST_WORD: usize = 16;

/// The pace at which what Claude writes is shown: a word at a time, never slower than the
/// teleprompter types (80 characters a second, about 13 words), and faster the more text
/// waits, so that what is shown stays within half a second of what arrived.
///
/// It reads no clock: each call is given the time, and the caller shows what it allows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cadence {
    allowance: f64,
    last: Option<Instant>,
}

impl Cadence {
    /// Returns how many of the `waiting` characters may be shown at `now`: what the time since
    /// the last call earned, at the pace `waiting` calls for, a tenth of a second at most, plus
    /// what was earned and not spent. Nothing waiting earns nothing.
    pub fn allowance(&mut self, waiting: usize, now: Instant) -> usize {
        let elapsed = self
            .last
            .replace(now)
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last))
            .min(LONGEST_STEP);
        if waiting == 0 {
            self.allowance = 0.0;
            return 0;
        }
        let waiting = f64::from(u32::try_from(waiting).unwrap_or(u32::MAX));
        let pace = (waiting / CATCH_UP).max(SLOWEST);
        self.allowance = (self.allowance + pace * elapsed.as_secs_f64()).min(waiting);
        whole(self.allowance)
    }

    /// Spends `shown` characters of the allowance.
    pub fn spend(&mut self, shown: usize) {
        let shown = f64::from(u32::try_from(shown).unwrap_or(u32::MAX));
        self.allowance = (self.allowance - shown).max(0.0);
    }

    /// Starts again from nothing: what was earned while something held the text back is lost.
    pub const fn rest(&mut self) {
        self.allowance = 0.0;
        self.last = None;
    }
}

/// Returns how many bytes of `text` to show within `allowance` characters: as many whole words
/// as fit, each with the spaces before it, and nothing of a word that does not fit; a word longer
/// than 16 characters (a path, a hash, code) is shown a part at a time.
#[must_use]
pub fn words_within(text: &str, allowance: usize) -> usize {
    let mut shown = 0;
    let mut room = allowance;
    let mut rest = text;
    while !rest.is_empty() {
        let spaces = rest.len() - rest.trim_start().len();
        let word = rest[spaces..]
            .find(char::is_whitespace)
            .map_or(rest.len(), |end| spaces + end);
        let length = columns(&rest[..word]);
        if length <= room {
            shown += word;
            room -= length;
            rest = &rest[word..];
            continue;
        }
        if length - columns(&rest[..spaces]) > LONGEST_WORD {
            shown += rest
                .char_indices()
                .nth(room)
                .map_or(rest.len(), |(at, _)| at);
        }
        break;
    }
    shown
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the allowance is floored, non-negative and never above the waiting count"
)]
fn whole(allowance: f64) -> usize {
    allowance.floor().max(0.0) as usize
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/reading/cadence.rs"]
mod tests;
