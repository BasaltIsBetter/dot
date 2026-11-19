use std::time::{Duration, Instant};

pub(crate) const SETTLE: Motion = Motion {
    duration: Duration::from_millis(300),
    curve: EASE_OUT,
};
pub(crate) const PIN: Motion = Motion {
    duration: Duration::from_millis(600),
    curve: CubicBezier {
        first_control: [0.2, 0.8],
        second_control: [0.2, 1.0],
    },
};
const EASE_OUT: CubicBezier = CubicBezier {
    first_control: [0.0, 0.0],
    second_control: [0.58, 1.0],
};
const SOLVER_STEPS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Motion {
    duration: Duration,
    curve: CubicBezier,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CubicBezier {
    first_control: [f64; 2],
    second_control: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Glide {
    from: f32,
    to: f32,
    since: Option<Instant>,
    motion: Motion,
}

impl Glide {
    pub(crate) const fn at_rest(position: f32, motion: Motion) -> Self {
        Self {
            from: position,
            to: position,
            since: None,
            motion,
        }
    }

    pub(crate) fn toward(&mut self, target: f32, now: Instant) {
        if (target - self.to).abs() < f32::EPSILON {
            return;
        }
        self.from = self.position(now);
        self.to = target;
        self.since = Some(now);
    }

    pub(crate) fn position(self, now: Instant) -> f32 {
        let eased = self
            .elapsed(now)
            .and_then(|elapsed| self.motion.eased(elapsed));
        let Some(eased) = eased else {
            return self.to;
        };
        self.from + (self.to - self.from) * to_f32(eased)
    }

    pub(crate) fn is_moving(self, now: Instant) -> bool {
        self.elapsed(now)
            .is_some_and(|elapsed| self.motion.lasts(elapsed))
    }

    pub(crate) const fn target(self) -> f32 {
        self.to
    }

    fn elapsed(self, now: Instant) -> Option<Duration> {
        Some(now.saturating_duration_since(self.since?))
    }
}

impl Motion {
    pub(crate) fn eased(self, elapsed: Duration) -> Option<f64> {
        let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        self.lasts(elapsed).then(|| self.curve.at(progress))
    }

    pub(crate) fn lasts(self, elapsed: Duration) -> bool {
        elapsed < self.duration
    }
}

impl CubicBezier {
    fn at(self, progress: f64) -> f64 {
        let [first_x, first_y] = self.first_control;
        let [second_x, second_y] = self.second_control;
        let mut low = 0.0;
        let mut high = 1.0;
        for _ in 0..SOLVER_STEPS {
            let middle = f64::midpoint(low, high);
            if coordinate(first_x, second_x, middle) < progress {
                low = middle;
            } else {
                high = middle;
            }
        }
        coordinate(first_y, second_y, f64::midpoint(low, high))
    }
}

pub(crate) fn ease_out(progress: f64) -> f64 {
    EASE_OUT.at(progress)
}

fn coordinate(first_control: f64, second_control: f64, along: f64) -> f64 {
    let before = 1.0 - along;
    3.0 * before * before * along * first_control
        + 3.0 * before * along * along * second_control
        + along * along * along
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "an eased share lies in [0, 1], far inside the range of f32"
)]
fn to_f32(share: f64) -> f32 {
    share as f32
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/ease.rs"]
mod tests;
