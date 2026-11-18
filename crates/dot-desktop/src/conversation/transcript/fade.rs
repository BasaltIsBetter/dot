use std::time::Duration;

const FRESH: Duration = Duration::from_millis(1_200);
const EASE_OUT_SECOND_X: f64 = 0.58;
const SOLVER_STEPS: u32 = 32;
const HOTTEST: f64 = 255.0;

pub(crate) fn is_fresh(elapsed: Duration) -> bool {
    elapsed < FRESH
}

pub(crate) fn heat(elapsed: Duration) -> u8 {
    if !is_fresh(elapsed) {
        return 0;
    }
    let progress = elapsed.as_secs_f64() / FRESH.as_secs_f64();
    to_heat(1.0 - ease_out(progress))
}

fn ease_out(progress: f64) -> f64 {
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..SOLVER_STEPS {
        let middle = f64::midpoint(low, high);
        if curve_x(middle) < progress {
            low = middle;
        } else {
            high = middle;
        }
    }
    curve_y(f64::midpoint(low, high))
}

fn curve_x(along: f64) -> f64 {
    3.0 * (1.0 - along) * along * along * EASE_OUT_SECOND_X + along * along * along
}

fn curve_y(along: f64) -> f64 {
    3.0 * along * along - 2.0 * along * along * along
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the share is clamped to [0, 1] before scaling to the range of u8"
)]
fn to_heat(share: f64) -> u8 {
    (share.clamp(0.0, 1.0) * HOTTEST).round() as u8
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/fade.rs"]
mod tests;
