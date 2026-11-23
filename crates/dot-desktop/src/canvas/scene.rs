use crate::canvas::layout::{Layout, TextSize};
use crate::canvas::mark::{Rectangle, Strip};
use dot_transcript::{Line, Placed, to_f32};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Scene {
    pub(crate) layout: Layout,
    pub(crate) lines: Vec<Line>,
    pub(crate) strip: Option<Strip>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) grounds: Vec<Rectangle>,
    pub(crate) marks: Vec<Rectangle>,
    pub(crate) veils: Vec<Rectangle>,
    pub(crate) animating: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pane {
    pub(crate) left: f32,
    pub(crate) size: TextSize,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
    pub(crate) lines: Vec<Placed>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Run<'a> {
    pub(crate) line: &'a Line,
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) clip: [f32; 2],
    pub(crate) size: TextSize,
}

impl Scene {
    pub(crate) fn fade_around(&mut self, [x, y]: [f32; 2], alpha: u8, scale: f64) {
        let share = f64::from(alpha) / f64::from(u8::MAX);
        let scale = to_f32(scale);
        let around = |value: f32, centre: f32| (centre + (value - centre) * scale).round();
        for pane in &mut self.panes {
            pane.left = around(pane.left, x);
            for placed in &mut pane.lines {
                placed.top = around(placed.top, y);
                placed.line = std::mem::take(&mut placed.line).faded(share);
            }
        }
        for rectangle in self.grounds.iter_mut().chain(&mut self.marks) {
            rectangle.left = around(rectangle.left, x);
            rectangle.top = around(rectangle.top, y);
            rectangle.color = rectangle.color.dimmed(alpha);
        }
    }

    pub(crate) fn runs(&self) -> Vec<Run<'_>> {
        let layout = &self.layout;
        let whole = [0.0, layout.height()];
        let grid = self.lines.iter().enumerate().map(|(index, line)| Run {
            line,
            left: layout.x(line.indent),
            top: layout.line_top(index),
            clip: whole,
            size: TextSize::Regular,
        });
        let panes = self.panes.iter().flat_map(|pane| {
            let advance = layout.advance_of(pane.size);
            pane.lines.iter().map(move |placed| Run {
                line: &placed.line,
                left: pane.left + f32::from(placed.line.indent) * advance,
                top: placed.top,
                clip: placed
                    .clip
                    .map_or([pane.top, pane.bottom], |[top, bottom]| {
                        [top.max(pane.top), bottom.min(pane.bottom)]
                    }),
                size: pane.size,
            })
        });
        let strip = self.strip.iter().map(|strip| Run {
            line: &strip.text,
            left: layout.strip_x(strip.text.indent),
            top: layout.strip_text_top(),
            clip: whole,
            size: TextSize::Small,
        });

        grid.chain(panes)
            .chain(strip)
            .filter(|run| run.line.has_text())
            .collect()
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/scene.rs"]
mod tests;
