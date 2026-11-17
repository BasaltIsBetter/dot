use winit::dpi::{PhysicalPosition, PhysicalSize};

const TEXT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;
const ADVANCE_PER_SIZE: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    scale: f32,
    left: f32,
    top: f32,
}

impl Layout {
    pub(crate) const COLUMNS: u16 = 64;
    pub(crate) const ROWS: usize = 11;
    pub(crate) const CENTRED_ROWS: usize = 8;

    pub(crate) fn new(size: PhysicalSize<u32>, scale_factor: f64) -> Self {
        let scale = to_f32(scale_factor);
        let [width, height] = [saturating_f32(size.width), saturating_f32(size.height)];

        Self {
            scale,
            left: centered(width, column_width(scale)),
            top: centered(height, rows_height(Self::CENTRED_ROWS, scale)),
        }
    }

    pub(crate) fn font_size(&self) -> f32 {
        TEXT_SIZE * self.scale
    }

    pub(crate) fn line_height(&self) -> f32 {
        LINE_HEIGHT * self.scale
    }

    pub(crate) fn x(&self, column: u16) -> f32 {
        self.left + f32::from(column) * self.font_size() * ADVANCE_PER_SIZE
    }

    pub(crate) fn line_top(&self, index: usize) -> f32 {
        self.top + saturating_f32(index) * self.line_height()
    }

    pub(crate) fn line_at(&self, position: PhysicalPosition<f64>) -> Option<usize> {
        let x = to_f32(position.x);
        let y = to_f32(position.y);
        if x < self.x(0) || x >= self.x(Self::COLUMNS) {
            return None;
        }

        (0..Self::ROWS).find(|&index| {
            let top = self.line_top(index);
            y >= top && y < top + self.line_height()
        })
    }
}

fn column_width(scale: f32) -> f32 {
    f32::from(Layout::COLUMNS) * TEXT_SIZE * ADVANCE_PER_SIZE * scale
}

fn rows_height(rows: usize, scale: f32) -> f32 {
    saturating_f32(rows) * LINE_HEIGHT * scale
}

fn centered(available: f32, needed: f32) -> f32 {
    ((available - needed) / 2.0).max(0.0).floor()
}

fn saturating_f32(value: impl TryInto<u16>) -> f32 {
    value.try_into().map_or(f32::from(u16::MAX), f32::from)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "scale factors and window coordinates stay far below the range of f32"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/layout.rs"]
mod tests;
