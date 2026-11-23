use std::iter;

use crate::canvas::layout::Layout;
use crate::canvas::mark::icon::Icon;
use crate::canvas::mark::shape::{Rectangle, Rgba};
use dot_transcript::{Ink, Line};

const TOP_RULE_INK: Ink = Ink::Trace;
const ZONE_RULE_INK: Ink = Ink::Faint;
const TRACK_INK: Ink = Ink::Trace;
const FILL_INK: Ink = Ink::Full;
const ICON_INK: Ink = Ink::Full;
const PER_MILLE: u16 = 1_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Strip {
    pub(crate) text: Line,
    pub(crate) rules: Vec<u16>,
    pub(crate) icons: Vec<(u16, Icon)>,
    pub(crate) gauge: Option<Gauge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Gauge {
    pub(crate) column: u16,
    pub(crate) fill_per_mille: u16,
}

impl Strip {
    pub(crate) fn rectangles(&self, layout: &Layout) -> Vec<Rectangle> {
        let hairline = Layout::HAIRLINE;
        let top_rule = Rectangle {
            left: 0.0,
            top: layout.strip_top(),
            width: layout.width(),
            height: hairline,
            color: Rgba::ink(TOP_RULE_INK),
        };
        let rule_height = layout.zone_rule_height();
        let rules = self.rules.iter().map(move |&column| Rectangle {
            left: layout.strip_x(column).round(),
            top: (layout.strip_middle() - rule_height / 2.0).round(),
            width: hairline,
            height: rule_height,
            color: Rgba::ink(ZONE_RULE_INK),
        });
        let icons = self
            .icons
            .iter()
            .flat_map(|&(column, icon)| icon.in_the_strip(layout, column, Rgba::ink(ICON_INK)));
        let gauge = self.gauge.iter().flat_map(|gauge| gauge.rectangles(layout));

        iter::once(top_rule)
            .chain(rules)
            .chain(icons)
            .chain(gauge)
            .collect()
    }
}

impl Gauge {
    fn rectangles(self, layout: &Layout) -> [Rectangle; 2] {
        let [width, height] = layout.gauge_size();
        let left = layout.strip_x(self.column).round();
        let top = (layout.strip_middle() - height / 2.0).round();
        let filled = f32::from(self.fill_per_mille.min(PER_MILLE)) / f32::from(PER_MILLE);

        [
            Rectangle {
                left,
                top,
                width,
                height,
                color: Rgba::ink(TRACK_INK),
            },
            Rectangle {
                left,
                top,
                width: (width * filled).round(),
                height,
                color: Rgba::ink(FILL_INK),
            },
        ]
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/strip.rs"]
mod tests;
