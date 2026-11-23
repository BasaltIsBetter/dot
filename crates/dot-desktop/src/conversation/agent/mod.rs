mod view;

use std::time::{Duration, Instant};

use dot_transcript::{Motion, opacity};

pub(crate) use self::view::agent_lines;

const FADE: Motion = Motion::landing(Duration::from_millis(450));
const OPENING_SCALE: f64 = 0.97;
const CLOSING_SCALE: f64 = 1.03;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentView {
    pub(crate) id: String,
    opened: Instant,
    scrolled: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fading {
    pub(crate) alpha: u8,
    pub(crate) scale: f64,
}

impl AgentView {
    pub(crate) const fn open(id: String, now: Instant) -> Self {
        Self {
            id,
            opened: now,
            scrolled: 0.0,
        }
    }

    pub(crate) fn fading(&self, now: Instant) -> Option<Fading> {
        fading(self.opened, OPENING_SCALE, now)
    }

    pub(crate) fn scroll_by(&mut self, rows_up: f64, rows: usize) {
        let most = f64::from(u32::try_from(rows).unwrap_or(u32::MAX));
        self.scrolled = (self.scrolled + rows_up).clamp(0.0, most);
    }

    pub(crate) const fn scrolled(&self) -> f64 {
        self.scrolled
    }
}

pub(crate) fn fading_back(closed: Instant, now: Instant) -> Option<Fading> {
    fading(closed, CLOSING_SCALE, now)
}

fn fading(since: Instant, scale: f64, now: Instant) -> Option<Fading> {
    let eased = FADE.eased(now.saturating_duration_since(since))?;
    Some(Fading {
        alpha: opacity(eased),
        scale: scale + (1.0 - scale) * eased,
    })
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/agent/mod.rs"]
mod tests;
