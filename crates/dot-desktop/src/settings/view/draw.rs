use dot_transcript::{Line, Placed, columns, index, rows, to_f32};

use crate::canvas::{Layout, Paint, Pane, Rectangle, TextSize};
use crate::settings::catalog::{ENTRIES, Entry};

pub(crate) const ROW: f32 = 30.0;
pub(crate) const HINT_SPACE: f32 = 26.0;
pub(crate) const HAIRLINE: f32 = 1.0;
pub(crate) const SECTION_GAP: f32 = 18.0;
const HALF_WIDTH: f32 = 300.0;
const SIDE: f32 = 40.0;
const MASK: f32 = 0.16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub(crate) left: f32,
    pub(crate) width: f32,
    pub(crate) band_top: f32,
    pub(crate) floor: f32,
}

impl Frame {
    pub(crate) fn of(layout: &Layout, centre: f32, floor: f32) -> Self {
        let half = layout
            .scaled(HALF_WIDTH)
            .min(layout.width() / 2.0 - layout.scaled(SIDE))
            .max(0.0);
        Self {
            left: (layout.width() / 2.0 - half).round(),
            width: (2.0 * half).round(),
            band_top: (centre - layout.scaled(ROW) / 2.0).round(),
            floor,
        }
    }

    pub(crate) fn right(&self) -> f32 {
        self.left + self.width
    }
}

pub(crate) fn offsets(layout: &Layout) -> Vec<f32> {
    let mut y = 0.0;
    ENTRIES
        .iter()
        .enumerate()
        .map(|(place, entry)| {
            if matches!(entry, Entry::Section(_)) && place > 0 {
                y += layout.scaled(SECTION_GAP);
            }
            let offset = y;
            y += layout.scaled(ROW);
            offset
        })
        .collect()
}

pub(crate) fn interpolated(offsets: &[f32], position: f64) -> f32 {
    let lower = position.floor().max(0.0);
    let share = to_f32((position - lower).clamp(0.0, 1.0));
    let at = |place: f64| {
        offsets
            .get(index(place))
            .or(offsets.last())
            .copied()
            .unwrap_or(0.0)
    };
    at(lower) + (at(lower + 1.0) - at(lower)) * share
}

pub(crate) fn below(place: usize, position: f64) -> f32 {
    to_f32((rows(place) - position).clamp(0.0, 1.0))
}

pub(crate) fn masked(middle: f32, floor: f32) -> f64 {
    if floor <= 0.0 {
        return 0.0;
    }
    let edge = floor * MASK;
    f64::from((middle / edge).min((floor - middle) / edge).clamp(0.0, 1.0))
}

pub(crate) fn ring([left, top, side]: [f32; 3], offset: f32, paint: Paint) -> [Rectangle; 4] {
    let outer = side + 2.0 * offset;
    let [left, top] = [left - offset, top - offset];
    [
        rectangle([left, top, outer, HAIRLINE], paint),
        rectangle([left, top + outer - HAIRLINE, outer, HAIRLINE], paint),
        rectangle([left, top, HAIRLINE, outer], paint),
        rectangle([left + outer - HAIRLINE, top, HAIRLINE, outer], paint),
    ]
}

pub(crate) fn pane(left: f32, size: TextSize, top: f32, line: Line) -> Pane {
    Pane {
        left: left.round(),
        size,
        top: f32::MIN,
        bottom: f32::MAX,
        lines: vec![Placed {
            top,
            line,
            clip: None,
        }],
    }
}

pub(crate) const fn rectangle([left, top, width, height]: [f32; 4], color: Paint) -> Rectangle {
    Rectangle {
        left,
        top,
        width,
        height,
        color,
    }
}

pub(crate) fn inside([x, y]: [f32; 2], [left, top, width, height]: [f32; 4]) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

pub(crate) fn width_of(text: &str) -> f32 {
    to_f32(rows(columns(text)))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/settings/view/draw.rs"]
mod tests;
