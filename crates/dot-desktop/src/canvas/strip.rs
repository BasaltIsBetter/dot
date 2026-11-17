use std::iter;

use crate::canvas::layout::Layout;
use crate::canvas::line::{Ink, Line};

const RULE_INK: Ink = Ink::Faint;
const TRACK_INK: Ink = Ink::Ground;
const PER_MILLE: u16 = 1_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Strip {
    pub(crate) text: Line,
    pub(crate) rules: Vec<u16>,
    pub(crate) dividers: Vec<u16>,
    pub(crate) gauge: Option<Gauge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Gauge {
    pub(crate) column: u16,
    pub(crate) width: u16,
    pub(crate) fill_per_mille: u16,
    pub(crate) ink: Ink,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rectangle {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ink: Ink,
}

impl Strip {
    pub(crate) fn rectangles(&self, layout: &Layout) -> Vec<Rectangle> {
        let hairline = Layout::HAIRLINE;
        let top_rule = Rectangle {
            left: 0.0,
            top: layout.strip_top(),
            width: layout.width(),
            height: hairline,
            ink: RULE_INK,
        };
        let rules = self.rules.iter().map(|&column| Rectangle {
            left: layout.strip_x(column).round(),
            top: layout.strip_text_top(),
            width: hairline,
            height: layout.line_height().round(),
            ink: RULE_INK,
        });
        let dividers = self.dividers.iter().map(|&column| Rectangle {
            left: layout.strip_x(column).round(),
            top: layout.strip_top(),
            width: hairline,
            height: layout.height() - layout.strip_top(),
            ink: RULE_INK,
        });
        let gauge = self.gauge.iter().flat_map(|gauge| gauge.rectangles(layout));

        iter::once(top_rule)
            .chain(rules)
            .chain(dividers)
            .chain(gauge)
            .collect()
    }
}

impl Gauge {
    fn rectangles(self, layout: &Layout) -> [Rectangle; 2] {
        let left = layout.strip_x(self.column).round();
        let right = layout
            .strip_x(self.column.saturating_add(self.width))
            .round();
        let width = right - left;
        let height = layout.gauge_height();
        let top = (layout.strip_text_top() + (layout.line_height() - height) / 2.0).round();
        let filled = f32::from(self.fill_per_mille.min(PER_MILLE)) / f32::from(PER_MILLE);

        [
            Rectangle {
                left,
                top,
                width,
                height,
                ink: TRACK_INK,
            },
            Rectangle {
                left,
                top,
                width: (width * filled).round(),
                height,
                ink: self.ink,
            },
        ]
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/strip.rs"]
mod tests;
