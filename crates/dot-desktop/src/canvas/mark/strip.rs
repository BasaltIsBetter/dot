use std::iter;

use crate::canvas::layout::Layout;
use crate::canvas::mark::icon::Icon;
use crate::canvas::mark::shape::{Paint, Rectangle};
use dot_transcript::{Ink, Line};

const TOP_RULE_INK: Ink = Ink::Trace;
const ZONE_RULE_INK: Ink = Ink::Faint;
const TRACK_INK: Ink = Ink::Trace;
const FILL_INK: Ink = Ink::Full;
const ICON_INK: Ink = Ink::Full;
const HOVERED_INK: Ink = Ink::Full;
const PER_MILLE: u16 = 1_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Strip {
    pub(crate) text: Line,
    pub(crate) rules: Vec<u16>,
    pub(crate) icons: Vec<(u16, Icon)>,
    pub(crate) gauge: Option<Gauge>,
    pub(crate) targets: Vec<(Target, [u16; 2])>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Model,
    Effort,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Gauge {
    pub(crate) column: u16,
    pub(crate) fill_per_mille: u16,
    pub(crate) tint: Option<[u8; 3]>,
}

impl Strip {
    pub(crate) fn rectangles(&self, layout: &Layout) -> Vec<Rectangle> {
        let hairline = Layout::HAIRLINE;
        let top_rule = Rectangle {
            left: 0.0,
            top: layout.strip_top(),
            width: layout.width(),
            height: hairline,
            color: Paint::ink(TOP_RULE_INK),
        };
        let rule_height = layout.zone_rule_height();
        let rules = self.rules.iter().map(move |&column| Rectangle {
            left: layout.strip_x(column).round(),
            top: (layout.strip_middle() - rule_height / 2.0).round(),
            width: hairline,
            height: rule_height,
            color: Paint::ink(ZONE_RULE_INK),
        });
        let icons = self
            .icons
            .iter()
            .flat_map(|&(column, icon)| icon.in_the_strip(layout, column, Paint::ink(ICON_INK)));
        let gauge = self.gauge.iter().flat_map(|gauge| gauge.rectangles(layout));

        iter::once(top_rule)
            .chain(rules)
            .chain(icons)
            .chain(gauge)
            .collect()
    }

    pub(crate) fn target_at(&self, layout: &Layout, pointer: [f32; 2]) -> Option<Target> {
        self.aimed(layout, pointer).map(|(target, _)| *target)
    }

    pub(crate) fn light_under(&mut self, layout: &Layout, pointer: [f32; 2]) {
        let Some(&(_, [start, end])) = self.aimed(layout, pointer) else {
            return;
        };
        let mut column = self.text.indent;
        for span in &mut self.text.spans {
            let width = u16::try_from(span.columns()).unwrap_or(u16::MAX);
            let after = column.saturating_add(width);
            let inside = column >= start && after <= end;
            if inside && !matches!(span.ink, Ink::Live | Ink::Rgb(_)) {
                span.ink = HOVERED_INK;
            }
            column = after;
        }
    }

    fn aimed(&self, layout: &Layout, [x, y]: [f32; 2]) -> Option<&(Target, [u16; 2])> {
        if y < layout.strip_top() {
            return None;
        }
        self.targets
            .iter()
            .find(|(_, [start, end])| x >= layout.strip_x(*start) && x < layout.strip_x(*end))
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
                color: Paint::ink(TRACK_INK),
            },
            Rectangle {
                left,
                top,
                width: (width * filled).round(),
                height,
                color: Paint::ink(self.tint.map_or(FILL_INK, Ink::Rgb)),
            },
        ]
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/strip.rs"]
mod tests;
