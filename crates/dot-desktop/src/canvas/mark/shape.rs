use dot_transcript::{Ink, OPAQUE, shade};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rectangle {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) color: Rgba,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rgba {
    pub(crate) rgb: [u8; 3],
    pub(crate) alpha: u8,
}

impl Rgba {
    pub(crate) const BLACK: [u8; 3] = [0, 0, 0];

    pub(crate) const fn ink(ink: Ink) -> Self {
        Self {
            rgb: ink.rgb(),
            alpha: OPAQUE,
        }
    }

    pub(crate) fn shaded(ink: Ink, heat: u8, alpha: u8) -> Self {
        Self {
            rgb: shade(ink, heat, alpha),
            alpha: OPAQUE,
        }
    }

    pub(crate) fn dimmed(self, alpha: u8) -> Self {
        let dim = |channel: u8| {
            let dimmed = (u16::from(channel) * u16::from(alpha) + u16::from(u8::MAX) / 2)
                / u16::from(u8::MAX);
            u8::try_from(dimmed).unwrap_or(u8::MAX)
        };
        Self {
            rgb: self.rgb.map(dim),
            alpha: self.alpha,
        }
    }

    pub(crate) const fn veil(alpha: u8) -> Self {
        Self {
            rgb: Self::BLACK,
            alpha,
        }
    }
}
