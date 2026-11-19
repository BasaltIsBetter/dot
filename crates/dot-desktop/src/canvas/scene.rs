use crate::canvas::layout::Layout;
use crate::canvas::line::Line;
use crate::canvas::mark::Strip;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Scene {
    pub(crate) layout: Layout,
    pub(crate) lines: Vec<Line>,
    pub(crate) strip: Option<Strip>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) animating: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pane {
    pub(crate) top: f32,
    pub(crate) bottom: f32,
    pub(crate) shift: f32,
    pub(crate) lines: Vec<Placed>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) top: f32,
    pub(crate) line: Line,
    pub(crate) clip: Option<[f32; 2]>,
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
}

impl Scene {
    pub(crate) fn runs(&self) -> Vec<Run<'_>> {
        let layout = &self.layout;
        let whole = [0.0, layout.height()];
        let grid = self.lines.iter().enumerate().map(|(index, line)| Run {
            line,
            left: layout.x(line.indent),
            top: layout.line_top(index),
            clip: whole,
        });
        let panes = self.panes.iter().flat_map(|pane| {
            pane.lines.iter().map(|placed| Run {
                line: &placed.line,
                left: layout.x(placed.line.indent) + pane.shift,
                top: placed.top,
                clip: placed
                    .clip
                    .map_or([pane.top, pane.bottom], |[top, bottom]| {
                        [top.max(pane.top), bottom.min(pane.bottom)]
                    }),
            })
        });
        let strip = self.strip.iter().map(|strip| Run {
            line: &strip.text,
            left: layout.strip_x(strip.text.indent),
            top: layout.strip_text_top(),
            clip: whole,
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
