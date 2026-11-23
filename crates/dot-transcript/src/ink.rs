/// Alpha of a line drawn with all of its ink.
pub const OPAQUE: u8 = 255;

const HOTTEST: u32 = 255;

/// Level of ink a piece of text is drawn in, white on black, or orange while live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ink {
    /// Full ink: what is in focus, the content.
    Full,
    /// 64 %: secondary text.
    Secondary,
    /// 40 %: what recedes, labels.
    Receding,
    /// 22 %: separators, what is disabled.
    Faint,
    /// 10 %: quiet backgrounds.
    Ground,
    /// 6 %: a band barely lighter than the black around it.
    Trace,
    /// Orange: a value that changes right now.
    Live,
}

impl Ink {
    /// Returns the sRGB color of the ink, on a black background.
    #[must_use]
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Full => [0xff, 0xff, 0xff],
            Self::Secondary => [0xa3, 0xa3, 0xa3],
            Self::Receding => [0x66, 0x66, 0x66],
            Self::Faint => [0x38, 0x38, 0x38],
            Self::Ground => [0x1a, 0x1a, 0x1a],
            Self::Trace => [0x0f, 0x0f, 0x0f],
            Self::Live => [0xff, 0x5a, 0x1f],
        }
    }
}

/// Returns the sRGB color of `ink` mixed toward orange by `heat`, then shown at `alpha` over the
/// black background: what the same ink at that opacity looks like on black, drawn opaque.
#[must_use]
pub fn shade(ink: Ink, heat: u8, alpha: u8) -> [u8; 3] {
    let settled = ink.rgb();
    let live = Ink::Live.rgb();
    std::array::from_fn(|channel| dim(mix(settled[channel], live[channel], heat), alpha))
}

/// Returns `share`, from 0 to 1, as an alpha from 0 to [`OPAQUE`]; outside the range, the nearest
/// end.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the share is clamped to [0, 1] first, so the rounded alpha fits in u8"
)]
pub fn opacity(share: f64) -> u8 {
    let share = if share.is_nan() { 0.0 } else { share };
    (share.clamp(0.0, 1.0) * f64::from(OPAQUE)).round() as u8
}

pub(crate) fn dim(channel: u8, alpha: u8) -> u8 {
    let dimmed =
        (u32::from(channel) * u32::from(alpha) + u32::from(OPAQUE) / 2) / u32::from(OPAQUE);
    u8::try_from(dimmed).unwrap_or(u8::MAX)
}

fn mix(settled: u8, live: u8, heat: u8) -> u8 {
    let heat = u32::from(heat);
    let blended =
        (u32::from(settled) * (HOTTEST - heat) + u32::from(live) * heat + HOTTEST / 2) / HOTTEST;
    u8::try_from(blended).unwrap_or(u8::MAX)
}

#[cfg(test)]
#[path = "../../../tests/dot-transcript/ink.rs"]
mod tests;
