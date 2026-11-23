use std::io::{self, Write};
use std::time::{Duration, Instant};

use super::flow::Flow;
use super::{Pace, Relay};

pub(super) struct Timing {
    pace: Pace,
    started: Instant,
    heard: Instant,
    told: Instant,
}

impl Timing {
    pub(super) const fn starting(pace: Pace, now: Instant) -> Self {
        Self {
            pace,
            started: now,
            heard: now,
            told: now,
        }
    }

    pub(super) const fn heard(&mut self, now: Instant) {
        self.heard = now;
    }

    pub(super) fn settle<R: Relay, W: Write>(
        &mut self,
        now: Instant,
        flow: &mut Flow<'_, R, W>,
    ) -> io::Result<Duration> {
        if flow.is_holding() {
            let left = self
                .pace
                .hold
                .saturating_sub(now.duration_since(self.started));
            if !left.is_zero() {
                return Ok(left);
            }
            flow.let_go()?;
            self.heard = now;
            self.told = now;
        }

        let silence_left = self
            .pace
            .pause
            .saturating_sub(now.duration_since(self.heard));
        let wait_left = self.pace.wait.saturating_sub(now.duration_since(self.told));
        let left = silence_left.min(wait_left);
        if !left.is_zero() {
            return Ok(left);
        }
        flow.pause()?;
        self.heard = now;
        self.told = now;
        Ok(self.pace.pause.min(self.pace.wait))
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/relay/timing.rs"]
mod tests;
