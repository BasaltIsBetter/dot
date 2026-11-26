use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use dot_transcript::{OPAQUE, opacity};

const PERIOD: Duration = Duration::from_millis(1_400);
const STEP: Duration = Duration::from_millis(50);
const DIMMEST: f64 = 0.45;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pulse {
    since: Option<Instant>,
}

impl Pulse {
    pub(crate) fn pace(&mut self, beating: bool, now: Instant) {
        if beating {
            self.since.get_or_insert(now);
        } else {
            self.since = None;
        }
    }

    pub(crate) fn alpha(&self, now: Instant) -> u8 {
        let Some(since) = self.since else {
            return OPAQUE;
        };
        let elapsed = now.saturating_duration_since(since).as_secs_f64();
        let phase = elapsed % PERIOD.as_secs_f64() / PERIOD.as_secs_f64();
        let dimming = (1.0 - (TAU * phase).cos()) / 2.0;
        opacity(1.0 - (1.0 - DIMMEST) * dimming)
    }

    pub(crate) fn next_change(&self, now: Instant) -> Option<Instant> {
        self.since.map(|_| now + STEP)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/agent/pulse.rs"]
mod tests;
