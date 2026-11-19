use winit::dpi::{PhysicalPosition, PhysicalSize};

const INTERFACE_ZOOM: f32 = 0.9;
const TEXT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;
const ADVANCE_PER_SIZE: f32 = 0.6;
const MARGIN: f32 = 24.0;
const SMALLEST_FIT: f32 = 0.5;
const STRIP_HEIGHT: f32 = 28.0;
const ICON_DOT_PER_SCALE: f32 = 1.5;
const STRIP_PADDING: f32 = 12.0;
const GAUGE_HEIGHT: f32 = 4.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    scale: f32,
    width: f32,
    height: f32,
    left: f32,
    top: f32,
    columns: u16,
}

impl Layout {
    pub(crate) const GRID_COLUMNS: u16 = 64;
    pub(crate) const WIDEST_COLUMN: u16 = 96;
    const NARROWEST_COLUMN: u16 = 30;
    pub(crate) const HAIRLINE: f32 = 1.0;
    pub(crate) const ROWS: usize = 11;
    pub(crate) const CENTRED_ROWS: usize = 8;

    pub(crate) fn zoomed(size: PhysicalSize<u32>, scale_factor: f64) -> Self {
        let screen_scale = interface_scale(scale_factor);
        let window = window_of(size);
        let [width, height] = window;
        let rows_below_centre = 2 * Self::ROWS - Self::CENTRED_ROWS;
        let margins = 2.0 * MARGIN * screen_scale;
        let fit = ((width - margins) / column_width(Self::GRID_COLUMNS, screen_scale))
            .min((height - margins) / rows_height(rows_below_centre, screen_scale))
            .clamp(SMALLEST_FIT, 1.0);

        Self::with_centred_column(window, screen_scale * fit, Self::GRID_COLUMNS)
    }

    pub(crate) fn reflowed(size: PhysicalSize<u32>, scale_factor: f64) -> Self {
        let scale = interface_scale(scale_factor);
        let window = window_of(size);
        let room = (window[0] - 2.0 * MARGIN * scale) / column_width(1, scale);
        let columns = whole_columns(room).clamp(Self::NARROWEST_COLUMN, Self::WIDEST_COLUMN);

        Self::with_centred_column(window, scale, columns)
    }

    fn with_centred_column([width, height]: [f32; 2], scale: f32, columns: u16) -> Self {
        Self {
            scale,
            width,
            height,
            left: centered(width, column_width(columns, scale)),
            top: centered(height, rows_height(Self::CENTRED_ROWS, scale)),
            columns,
        }
    }

    pub(crate) const fn columns(&self) -> u16 {
        self.columns
    }

    pub(crate) fn font_size(&self) -> f32 {
        TEXT_SIZE * self.scale
    }

    pub(crate) fn line_height(&self) -> f32 {
        LINE_HEIGHT * self.scale
    }

    pub(crate) fn width(&self) -> f32 {
        self.width
    }

    pub(crate) fn height(&self) -> f32 {
        self.height
    }

    pub(crate) fn x(&self, column: u16) -> f32 {
        self.left + f32::from(column) * self.advance()
    }

    pub(crate) fn line_top(&self, index: usize) -> f32 {
        self.top + saturating_f32(index) * self.line_height()
    }

    pub(crate) fn line_at(&self, position: PhysicalPosition<f64>) -> Option<usize> {
        let x = to_f32(position.x);
        let y = to_f32(position.y);
        if x < self.x(0) || x >= self.x(self.columns) {
            return None;
        }

        (0..Self::ROWS).find(|&index| {
            let top = self.line_top(index);
            y >= top && y < top + self.line_height()
        })
    }

    pub(crate) fn strip_top(&self) -> f32 {
        (self.height - STRIP_HEIGHT * self.scale).max(0.0).floor()
    }

    pub(crate) fn strip_text_top(&self) -> f32 {
        self.strip_top() + ((STRIP_HEIGHT - LINE_HEIGHT) / 2.0 * self.scale).floor()
    }

    pub(crate) fn strip_x(&self, column: u16) -> f32 {
        self.strip_padding() + f32::from(column) * self.advance()
    }

    pub(crate) fn strip_columns(&self) -> u16 {
        whole_columns((self.width - 2.0 * self.strip_padding()) / self.advance())
    }

    pub(crate) fn gauge_height(&self) -> f32 {
        (GAUGE_HEIGHT * self.scale).round().max(1.0)
    }

    pub(crate) fn icon_dot(&self) -> f32 {
        (self.font_size() / TEXT_SIZE * ICON_DOT_PER_SCALE)
            .round()
            .max(1.0)
    }

    pub(crate) fn advance(&self) -> f32 {
        self.font_size() * ADVANCE_PER_SIZE
    }

    pub(crate) fn scaled(&self, points: f32) -> f32 {
        (points * self.scale).round()
    }

    fn strip_padding(&self) -> f32 {
        (STRIP_PADDING * self.scale).floor()
    }
}

fn interface_scale(scale_factor: f64) -> f32 {
    to_f32(scale_factor) * INTERFACE_ZOOM
}

fn window_of(size: PhysicalSize<u32>) -> [f32; 2] {
    [saturating_f32(size.width), saturating_f32(size.height)]
}

fn column_width(columns: u16, scale: f32) -> f32 {
    f32::from(columns) * TEXT_SIZE * ADVANCE_PER_SIZE * scale
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
pub(crate) fn to_f32(value: f64) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the count is floored and clamped to the range of u16 before the cast"
)]
fn whole_columns(count: f32) -> u16 {
    count.floor().clamp(0.0, f32::from(u16::MAX)) as u16
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/layout.rs"]
mod tests;
