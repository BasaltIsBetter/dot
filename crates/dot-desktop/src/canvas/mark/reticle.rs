use winit::dpi::PhysicalPosition;

use crate::canvas::layout::{Layout, to_f32};
use crate::canvas::line::Ink;
use crate::canvas::mark::shape::{Rectangle, Ring};

const RING_RADIUS: f32 = 8.0;
const RING_INK: Ink = Ink::Full;
const HAIRLINE_INK: Ink = Ink::Ground;
const PIXEL_CENTRE: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reticle {
    column: f32,
    row: f32,
    radius: f32,
}

impl Reticle {
    pub(crate) fn at(position: PhysicalPosition<f64>, scale_factor: f64) -> Self {
        Self {
            column: to_f32(position.x).floor(),
            row: to_f32(position.y).floor(),
            radius: (RING_RADIUS * to_f32(scale_factor)).round(),
        }
    }

    pub(crate) fn hairlines(&self, width: f32, bottom: f32) -> impl Iterator<Item = Rectangle> {
        let hairline = Layout::HAIRLINE;
        let below = self.row + self.radius + hairline;
        let right = self.column + self.radius + hairline;
        let vertical = |top: f32, bottom: f32| Rectangle {
            left: self.column,
            top,
            width: hairline,
            height: bottom - top,
            ink: HAIRLINE_INK,
        };
        let horizontal = |left: f32, end: f32| Rectangle {
            left,
            top: self.row,
            width: end - left,
            height: hairline,
            ink: HAIRLINE_INK,
        };

        [
            vertical(0.0, (self.row - self.radius).min(bottom)),
            vertical(below, bottom),
            horizontal(0.0, self.column - self.radius),
            horizontal(right, width),
        ]
        .into_iter()
        .filter(move |hairline| {
            hairline.width > 0.0 && hairline.height > 0.0 && hairline.top < bottom
        })
    }

    pub(crate) fn ring(&self) -> Ring {
        Ring {
            centre_x: self.column + PIXEL_CENTRE,
            centre_y: self.row + PIXEL_CENTRE,
            radius: self.radius,
            thickness: Layout::HAIRLINE,
            ink: RING_INK,
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/reticle.rs"]
mod tests;
