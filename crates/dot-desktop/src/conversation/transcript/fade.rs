use std::time::Duration;

use crate::canvas::ease_out;

const FRESH: Duration = Duration::from_millis(1_200);
const HOTTEST_SHARE: f64 = 255.0;

pub(crate) const HOTTEST: u8 = u8::MAX;

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

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the share is clamped to [0, 1] before scaling to the range of u8"
)]
fn to_heat(share: f64) -> u8 {
    (share.clamp(0.0, 1.0) * HOTTEST_SHARE).round() as u8
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/fade.rs"]
mod tests;
