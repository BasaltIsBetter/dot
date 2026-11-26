use std::time::Instant;

use dot_api::{Text, Translator};

use super::{Side, Sighting};
use crate::canvas::{Layout, Paint, Pane, Rectangle, Silhouette, TextSize};
use crate::conversation::flow::Conversation;
use crate::conversation::stage::Stage;
use dot_transcript::{Ink, Line, Placed, Span, columns, index, opacity, truncated};

const FROM_THE_TOP: f32 = 16.0;
const FROM_THE_BOTTOM: f32 = 64.0;
const GAP: f32 = 6.0;
const ABOVE: &str = "↑";
const BELOW: &str = "↓";
const SEPARATOR: &str = " · ";
const INK: Ink = Ink::Secondary;
const HOVERED_INK: Ink = Ink::Full;

pub(super) struct Placement {
    arrow: &'static str,
    text: String,
    left: f32,
    top: f32,
    figure_left: f32,
    text_left: f32,
    right: f32,
    bottom: f32,
}

impl Conversation {
    pub(in crate::conversation::flow) fn draw_marker(
        &self,
        (panes, marks): (&mut Vec<Pane>, &mut Vec<Rectangle>),
        (stage, layout): (&Stage, &Layout),
        silhouette: &Silhouette,
        now: Instant,
        translator: &impl Translator,
    ) {
        let shown = f64::from(self.marker.shown.position(now));
        let Some(sighting) = self.marker.last.as_ref().filter(|_| shown > 0.0) else {
            return;
        };
        let placed = placement(sighting, (stage, layout), silhouette.size(), translator);
        clear_behind(&placed, (panes, marks), (stage, layout), 1.0 - shown);

        let hovered = self.pointer.is_some_and(|point| placed.holds(point));
        let ink = if hovered { HOVERED_INK } else { INK };
        let text_top = layout.text_top(TextSize::Small, placed.top, stage.row);
        let pane = |left: f32, text: &str| Pane {
            left,
            size: TextSize::Small,
            top: 0.0,
            bottom: stage.floor,
            lines: vec![Placed {
                top: text_top,
                line: Line::new(0, vec![Span::new(text, ink)]).faded(shown),
                clip: None,
            }],
        };
        panes.push(pane(placed.left, placed.arrow));
        panes.push(pane(placed.text_left, &placed.text));
        let figure_top = placed.top + (stage.row - silhouette.size()[1]) / 2.0;
        let paint = Paint::ink(self.colours.tool.map_or(Ink::Live, Ink::Rgb))
            .dimmed(opacity(shown))
            .dimmed(self.pulse.alpha(now));
        marks.extend(silhouette.drawn([placed.figure_left, figure_top], paint));
    }
}

impl Placement {
    pub(super) fn holds(&self, [x, y]: [f32; 2]) -> bool {
        (self.left..self.right).contains(&x) && (self.top..self.bottom).contains(&y)
    }

    fn hides(&self, [left, right]: [f32; 2], down: [f32; 2], margin: f32) -> bool {
        left < self.right + margin && right > self.left - margin && self.crosses(down)
    }

    fn crosses(&self, [top, bottom]: [f32; 2]) -> bool {
        top < self.bottom && bottom > self.top
    }
}

pub(super) fn placement(
    sighting: &Sighting,
    (stage, layout): (&Stage, &Layout),
    [figure_width, _]: [f32; 2],
    translator: &impl Translator,
) -> Placement {
    let advance = layout.advance_of(TextSize::Small);
    let width_of = |text: &str| to_f32(columns(text)) * advance;
    let [column_left, column_right] = stage.column;
    let gap = stage.points(GAP);
    let arrow = match sighting.side {
        Side::Above => ABOVE,
        Side::Below => BELOW,
    };
    let leading = width_of(arrow) + gap + figure_width + gap;
    let counted = translator.count(Text::AgentsOutOfSight, sighting.count);
    let room = index(f64::from(
        ((column_right - column_left - leading) / advance).max(0.0),
    ));
    let text = truncated(&format!("{counted}{SEPARATOR}{}", sighting.names), room);
    let width = leading + width_of(&text);
    let left = ((column_left + column_right - width) / 2.0).round();
    let top = match sighting.side {
        Side::Above => stage.points(FROM_THE_TOP),
        Side::Below => (stage.height - stage.points(FROM_THE_BOTTOM) - stage.row).round(),
    };
    Placement {
        arrow,
        text,
        left,
        top,
        figure_left: left + width_of(arrow) + gap,
        text_left: left + leading,
        right: left + width,
        bottom: top + stage.row,
    }
}

fn clear_behind(
    placed: &Placement,
    (panes, marks): (&mut [Pane], &mut [Rectangle]),
    (stage, layout): (&Stage, &Layout),
    left: f64,
) {
    let margin = stage.points(GAP);
    for pane in panes {
        let advance = layout.advance_of(pane.size);
        let height = layout.line_height_of(pane.size);
        let behind = pane
            .lines
            .iter_mut()
            .filter(|line| placed.crosses([line.top, line.top + height]));
        for line in behind {
            let start = pane.left + f32::from(line.line.indent) * advance;
            let width: usize = line.line.spans.iter().map(Span::columns).sum();
            let across = [start, start + to_f32(width) * advance];
            if placed.hides(across, [line.top, line.top + height], margin) {
                let shown = std::mem::replace(&mut line.line, Line::new(0, Vec::new()));
                line.line = shown.faded(left);
            }
        }
    }
    for mark in marks {
        let across = [mark.left, mark.left + mark.width];
        if placed.hides(across, [mark.top, mark.top + mark.height], margin) {
            mark.color = mark.color.dimmed(opacity(left));
        }
    }
}

fn to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[cfg(test)]
#[path = "../../../../../../../tests/dot-desktop/conversation/flow/running/marker/draw.rs"]
mod tests;
