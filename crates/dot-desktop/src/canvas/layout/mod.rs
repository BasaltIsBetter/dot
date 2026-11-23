mod room;
mod strip;
mod text;

use dot_transcript::to_f32;
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub(crate) use self::room::Room;
pub(crate) use self::text::TextSize;

const INTERFACE_ZOOM: f32 = 0.8;
const TEXT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;
const READING_LINE_HEIGHT: f32 = 22.0;
const ADVANCE_PER_SIZE: f32 = 0.6;
const MARGIN: f32 = 24.0;
const SMALLEST_FIT: f32 = 0.5;
const ICON_DOT_PER_SCALE: f32 = 1.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    scale: f32,
    width: f32,
    height: f32,
    left: f32,
    top: f32,
    columns: u16,
    line_points: f32,
    room: Room,
}

impl Layout {
    pub(crate) const GRID_COLUMNS: u16 = 64;
    pub(crate) const WIDEST_COLUMN: u16 = dot_transcript::WIDEST_COLUMN;
    const NARROWEST_COLUMN: u16 = 24;
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
        let room = Room::of(window[0] / scale);
        let reserves = room.reserves().map(|reserve| reserve * scale);
        let usable = window[0] - reserves[0] - reserves[1];
        let columns = whole_columns(usable / column_width(1, scale))
            .clamp(Self::NARROWEST_COLUMN, Self::WIDEST_COLUMN);
        let free = window[0] - column_width(columns, scale);
        let centred = Self::with_centred_column(window, scale, columns);

        Self {
            left: (centred.left + room::shift(free, reserves))
                .max(0.0)
                .floor(),
            line_points: READING_LINE_HEIGHT,
            room,
            ..centred
        }
    }

    fn with_centred_column([width, height]: [f32; 2], scale: f32, columns: u16) -> Self {
        Self {
            scale,
            width,
            height,
            left: centered(width, column_width(columns, scale)),
            top: centered(height, rows_height(Self::CENTRED_ROWS, scale)),
            columns,
            line_points: LINE_HEIGHT,
            room: Room::default(),
        }
    }

    pub(crate) const fn room(&self) -> Room {
        self.room
    }

    pub(crate) const fn scale(&self) -> f32 {
        self.scale
    }

    pub(crate) const fn columns(&self) -> u16 {
        self.columns
    }

    pub(crate) fn font_size(&self) -> f32 {
        TEXT_SIZE * self.scale
    }

    pub(crate) fn line_height(&self) -> f32 {
        self.line_points * self.scale
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
    clippy::cast_sign_loss,
    reason = "the count is floored and clamped to the range of u16 before the cast"
)]
pub(super) fn whole_columns(count: f32) -> u16 {
    count.floor().clamp(0.0, f32::from(u16::MAX)) as u16
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/layout/mod.rs"]
mod tests;
