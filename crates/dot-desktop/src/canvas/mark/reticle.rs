use winit::dpi::PhysicalPosition;

use dot_transcript::{Ink, to_f32};

use crate::canvas::layout::Layout;
use crate::canvas::mark::shape::{Paint, Rectangle};

const HALF_SIDE: f32 = 8.0;
const FRAME_INK: Ink = Ink::Full;
const HAIRLINE_INK: Ink = Ink::Ground;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reticle {
    column: f32,
    row: f32,
    half_side: f32,
}

impl Reticle {
    pub(crate) fn at(position: PhysicalPosition<f64>, scale_factor: f64) -> Self {
        Self {
            column: to_f32(position.x).floor(),
            row: to_f32(position.y).floor(),
            half_side: (HALF_SIDE * to_f32(scale_factor)).round(),
        }
    }

    pub(crate) fn hairlines(&self, width: f32, bottom: f32) -> impl Iterator<Item = Rectangle> {
        let hairline = Layout::HAIRLINE;
        let below = self.row + self.half_side + hairline;
        let right = self.column + self.half_side + hairline;
        let vertical = |top: f32, bottom: f32| Rectangle {
            left: self.column,
            top,
            width: hairline,
            height: bottom - top,
            color: Paint::ink(HAIRLINE_INK),
        };
        let horizontal = |left: f32, end: f32| Rectangle {
            left,
            top: self.row,
            width: end - left,
            height: hairline,
            color: Paint::ink(HAIRLINE_INK),
        };

        [
            vertical(0.0, (self.row - self.half_side).min(bottom)),
            vertical(below, bottom),
            horizontal(0.0, self.column - self.half_side),
            horizontal(right, width),
        ]
        .into_iter()
        .filter(move |hairline| {
            hairline.width > 0.0 && hairline.height > 0.0 && hairline.top < bottom
        })
    }

    pub(crate) fn frame(&self) -> [Rectangle; 4] {
        let hairline = Layout::HAIRLINE;
        let left = self.column - self.half_side;
        let top = self.row - self.half_side;
        let side = 2.0 * self.half_side + hairline;
        let edge = |left: f32, top: f32, width: f32, height: f32| Rectangle {
            left,
            top,
            width,
            height,
            color: Paint::ink(FRAME_INK),
        };
        let inner = side - 2.0 * hairline;

        [
            edge(left, top, side, hairline),
            edge(left, self.row + self.half_side, side, hairline),
            edge(left, top + hairline, hairline, inner),
            edge(
                self.column + self.half_side,
                top + hairline,
                hairline,
                inner,
            ),
        ]
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/reticle.rs"]
mod tests;
