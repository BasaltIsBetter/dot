use dot_transcript::{Band, Ink, TEXT_INDENT, TextWidth, index, opacity, rows, to_f32};

use crate::canvas::{Layout, Paint, Rectangle};

const BOTTOM_CLEARANCE: f32 = 64.0;
const BAND_HALF_HEIGHT: f32 = 15.0;
const TICK_LEFT: f32 = 16.0;
const TICK_WIDTH: f32 = 16.0;
const BAND_OVERHANG: f32 = 24.0;
const BAND_RAIL_GAP: f32 = 32.0;
const GUTTER_GAP: f32 = 16.0;
const GUTTER_WIDTH: f32 = 160.0;
const RAIL_MARGIN: f32 = 24.0;
const RAIL_WIDTH: f32 = 200.0;
const FADE_HEIGHT: f32 = 32.0;
const BAND_INK: Ink = Ink::Trace;
const TICK_INK: Ink = Ink::Receding;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stage {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) row: f32,
    pub(crate) centre: f32,
    pub(crate) floor: f32,
    pub(crate) column: [f32; 2],
    pub(crate) gutter: Option<[f32; 2]>,
    pub(crate) rail: Option<[f32; 2]>,
    scale: f32,
}

impl Stage {
    pub(crate) fn of(layout: &Layout) -> Self {
        let scale = layout.scale();
        let scaled = |points: f32| layout.scaled(points);
        let column = [layout.x(0), layout.x(layout.columns())];
        let room = layout.room();
        let gutter = room
            .has_gutter()
            .then(|| column[1] + scaled(GUTTER_GAP))
            .map(|left| [left, left + scaled(GUTTER_WIDTH)]);
        let rail = room.has_rail().then(|| {
            let right = layout.width() - scaled(RAIL_MARGIN);
            [right - scaled(RAIL_WIDTH), right]
        });
        Self {
            width: layout.width(),
            height: layout.height(),
            row: layout.line_height(),
            centre: layout.reading_line(),
            floor: layout.strip_top(),
            column,
            gutter,
            rail,
            scale,
        }
    }

    pub(crate) fn middle(&self) -> [f32; 2] {
        [(self.width / 2.0).round(), self.centre]
    }

    pub(crate) const fn band(&self) -> Band {
        Band {
            top: 0.0,
            bottom: self.floor,
        }
    }

    pub(crate) fn top_row(&self, focus: f64) -> f64 {
        focus - f64::from((self.centre - self.row / 2.0) / self.row)
    }

    pub(crate) fn row_top(&self, row: usize, top_row: f64) -> f32 {
        self.band().row_top(rows(row) - top_row, self.row)
    }

    pub(crate) fn row_at(&self, y: f32, focus: f64) -> Option<usize> {
        let row = self.top_row(focus) + f64::from(y / self.row);
        (row >= 0.0 && y < self.floor).then(|| index(row.floor()))
    }

    pub(crate) fn rows_below(&self) -> usize {
        let room = self.lowest_row_bottom() - self.centre - self.row / 2.0;
        index(f64::from((room / self.row).floor()))
    }

    pub(crate) fn lowest_row_bottom(&self) -> f32 {
        self.height - self.points(BOTTOM_CLEARANCE)
    }

    pub(crate) fn reading_band(&self, rows_away: f64) -> Rectangle {
        let lowest = self.lowest_row_bottom() - self.centre;
        let highest_up = self.row - self.centre;
        let offset =
            (to_f32(rows_away) * self.row).clamp(highest_up, lowest.max(0.0).max(highest_up));
        let half = self.points(BAND_HALF_HEIGHT);
        Rectangle {
            left: 0.0,
            top: (self.centre - half + offset).round(),
            width: self.band_end(),
            height: 2.0 * half,
            color: Paint::ink(BAND_INK),
        }
    }

    pub(crate) fn tick(&self, band: &Rectangle) -> Rectangle {
        Rectangle {
            left: self.points(TICK_LEFT),
            top: (band.top + band.height / 2.0).floor(),
            width: self.points(TICK_WIDTH),
            height: Layout::HAIRLINE,
            color: Paint::ink(TICK_INK),
        }
    }

    pub(crate) fn veils(&self) -> Vec<Rectangle> {
        let fade = self.points(FADE_HEIGHT);
        let from = (self.floor - fade).max(0.0);
        let strips = index(f64::from(self.floor - from));
        (0..strips)
            .map(|strip| {
                let top = from + to_f32(rows(strip));
                Rectangle {
                    left: 0.0,
                    top,
                    width: self.width,
                    height: 1.0,
                    color: Paint::veil(opacity(f64::from((top + 0.5 - from) / fade))),
                }
            })
            .collect()
    }

    pub(crate) fn points(&self, points: f32) -> f32 {
        (points * self.scale).round()
    }

    pub(crate) fn exact_points(&self, points: f32) -> f32 {
        points * self.scale
    }

    fn band_end(&self) -> f32 {
        match self.rail {
            Some([left, _]) => left - self.points(BAND_RAIL_GAP),
            None => self.column[1] + self.points(BAND_OVERHANG),
        }
    }
}

pub(crate) fn rows_in_sight(
    count: usize,
    top_of: impl Fn(usize) -> f32,
    row: f32,
    height: f32,
) -> std::ops::Range<usize> {
    let first = first_row_where(count, |at| top_of(at) + row >= 0.0);
    let end = first_row_where(count, |at| top_of(at) > height);
    first..end.max(first)
}

fn first_row_where(count: usize, holds: impl Fn(usize) -> bool) -> usize {
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = low + (high - low) / 2;
        if holds(middle) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

pub(crate) fn text_width(layout: &Layout) -> TextWidth {
    TextWidth::new(usize::from(layout.columns().saturating_sub(TEXT_INDENT)))
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/stage.rs"]
mod tests;
