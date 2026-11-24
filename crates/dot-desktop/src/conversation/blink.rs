use std::time::{Duration, Instant};

const HALF_PERIOD: Duration = Duration::from_millis(530);

#[derive(Debug, Default)]
pub(crate) struct Blink {
    since: Option<Instant>,
    steady: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Caret {
    Lit,
    Dark,
    Receded,
}

impl Caret {
    pub(crate) const fn is_shown(self) -> bool {
        !matches!(self, Self::Dark)
    }
}

impl Blink {
    pub(crate) fn pace(&mut self, focused: bool, now: Instant) {
        if focused {
            self.since.get_or_insert(now);
        } else {
            self.since = None;
        }
    }

    pub(crate) const fn hold(&mut self, steady: bool) {
        self.steady = steady;
    }

    pub(crate) fn restart(&mut self, now: Instant) {
        if self.since.is_some() {
            self.since = Some(now);
        }
    }

    pub(crate) fn caret(&self, now: Instant) -> Caret {
        let Some(since) = self.since else {
            return Caret::Receded;
        };
        if self.steady || half_periods(since, now).is_multiple_of(2) {
            Caret::Lit
        } else {
            Caret::Dark
        }
    }

    pub(crate) fn next_change(&self, now: Instant) -> Option<Instant> {
        let since = self.since.filter(|_| !self.steady)?;
        let next = half_periods(since, now).saturating_add(1);
        let elapsed = HALF_PERIOD.saturating_mul(u32::try_from(next).unwrap_or(u32::MAX));
        Some(since + elapsed)
    }
}

fn half_periods(since: Instant, now: Instant) -> u128 {
    now.saturating_duration_since(since).as_nanos() / HALF_PERIOD.as_nanos()
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/blink.rs"]
mod tests;
