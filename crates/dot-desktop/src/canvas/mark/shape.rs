use dot_transcript::{Ink, OPAQUE, Palette};

use crate::canvas::Layout;

const LINE: f32 = Layout::HAIRLINE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rectangle {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) color: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Paint {
    pub(crate) ink: Ink,
    pub(crate) heat: u8,
    pub(crate) dim: u8,
    pub(crate) alpha: u8,
}

impl Rectangle {
    pub(crate) fn clip_to(&mut self, band: [f32; 2]) -> bool {
        clip_span([&mut self.top, &mut self.height], band)
    }
}

impl Paint {
    const BLACK: Ink = Ink::Rgb([0, 0, 0]);

    pub(crate) const fn ink(ink: Ink) -> Self {
        Self::shaded(ink, 0, OPAQUE)
    }

    pub(crate) const fn shaded(ink: Ink, heat: u8, dim: u8) -> Self {
        Self {
            ink,
            heat,
            dim,
            alpha: OPAQUE,
        }
    }

    pub(crate) fn dimmed(self, alpha: u8) -> Self {
        let dimmed =
            (u16::from(self.dim) * u16::from(alpha) + u16::from(u8::MAX) / 2) / u16::from(u8::MAX);
        Self {
            dim: u8::try_from(dimmed).unwrap_or(u8::MAX),
            ..self
        }
    }

    pub(crate) const fn veil(alpha: u8) -> Self {
        Self {
            alpha,
            ..Self::ink(Self::BLACK)
        }
    }

    pub(crate) fn rgb_in(self, palette: Palette) -> [u8; 3] {
        palette.shade(self.ink, self.heat, self.dim)
    }

    #[cfg(test)]
    pub(crate) fn rgb(self) -> [u8; 3] {
        self.rgb_in(Palette::default())
    }
}

pub(crate) fn clip_span([top, height]: [&mut f32; 2], [low, high]: [f32; 2]) -> bool {
    let bottom = (*top + *height).min(high);
    *top = top.max(low);
    *height = bottom - *top;
    *height > 0.0
}

pub(crate) fn corners(
    [left, top, width, height]: [f32; 4],
    arm: f32,
    colour: Paint,
) -> [Rectangle; 8] {
    let [right, bottom] = [left + width, top + height];
    let [across, down] = [arm.min(width), arm.min(height)];
    let rectangle = |left: f32, top: f32, width: f32, height: f32| Rectangle {
        left: left.round(),
        top: top.round(),
        width,
        height,
        color: colour,
    };
    [
        rectangle(left, top, across, LINE),
        rectangle(left, top, LINE, down),
        rectangle(right - across, top, across, LINE),
        rectangle(right - LINE, top, LINE, down),
        rectangle(left, bottom - LINE, across, LINE),
        rectangle(left, bottom - down, LINE, down),
        rectangle(right - across, bottom - LINE, across, LINE),
        rectangle(right - LINE, bottom - down, LINE, down),
    ]
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/shape.rs"]
mod tests;
