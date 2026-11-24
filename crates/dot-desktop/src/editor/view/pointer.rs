use std::time::Instant;

use dot_api::Translator;
use dot_transcript::index;

use crate::canvas::Layout;
use crate::editor::Editor;
use crate::editor::view::bar::{Bar, Hit};
use crate::editor::view::{Frame, Pressed};

const EDGE: f32 = 24.0;
const EDGE_ROWS_PER_FRAME: f64 = 0.35;

impl Editor {
    pub(crate) fn press(
        &mut self,
        [x, y]: [f32; 2],
        (extend, clicks, now): (bool, u8, Instant),
        layout: &Layout,
        translator: &impl Translator,
    ) -> Pressed {
        self.held = None;
        let frame = self.frame(layout);
        match Bar::of(&frame, layout, translator).hit(self, [x, y], &frame) {
            Some(Hit::Back) => return changed(self.step(true, now)),
            Some(Hit::Forward) => return changed(self.step(false, now)),
            Some(Hit::Close) => return Pressed::Close,
            Some(Hit::Open(path)) => return Pressed::Open(path),
            None => {}
        }
        if x < frame.pane || y < frame.code_top || y >= frame.bottom {
            return Pressed::Nothing;
        }
        let row = self.top + index(f64::from(((y - frame.code_top) / frame.row).floor()));
        if row >= self.buffer.rows().len() {
            return Pressed::Nothing;
        }
        let column = column_at(&frame, x);
        let held = match clicks {
            0 | 1 => {
                self.buffer.place(row, column, extend);
                let fixed = self.buffer.fixed_end();
                fixed..fixed
            }
            2 => self.buffer.select_word(row, column),
            _ => self.buffer.select_line(row, column),
        };
        self.held = Some(held);
        Pressed::Placed
    }

    pub(crate) fn drag(&mut self, [x, y]: [f32; 2], layout: &Layout) {
        let Some(held) = self.held.clone() else {
            return;
        };
        let frame = self.frame(layout);
        let shown = index(f64::from(
            ((y - frame.code_top) / frame.row).floor().max(0.0),
        ));
        let last = self.buffer.rows().len().saturating_sub(1);
        let lowest = self.top.saturating_add(frame.visible() - 1);
        let row = self.top.saturating_add(shown).min(lowest).min(last);
        self.buffer.drag_to(row, column_at(&frame, x), &held);
    }

    pub(crate) fn follow_edges(&mut self, at: [f32; 2], layout: &Layout) -> bool {
        if self.held.is_none() {
            return false;
        }
        let frame = self.frame(layout);
        let edge = layout.scaled(EDGE);
        let rows = if at[1] < frame.code_top + edge {
            -EDGE_ROWS_PER_FRAME
        } else if at[1] > frame.bottom - edge {
            EDGE_ROWS_PER_FRAME
        } else {
            self.edge = 0.0;
            return false;
        };
        self.edge += rows;
        let whole = self.edge.trunc();
        if whole != 0.0 {
            self.scroll(whole);
            self.edge -= whole;
        }
        self.drag(at, layout);
        true
    }

    pub(crate) fn release(&mut self) {
        self.held = None;
        self.edge = 0.0;
    }
}

fn column_at(frame: &Frame, x: f32) -> usize {
    index(f64::from(
        ((x - frame.code) / frame.advance).round().max(0.0),
    ))
}

const fn changed(stepped: bool) -> Pressed {
    if stepped {
        Pressed::Changed
    } else {
        Pressed::Nothing
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/view/pointer.rs"]
mod tests;
