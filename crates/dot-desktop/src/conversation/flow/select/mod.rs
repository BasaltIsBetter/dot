mod draw;

use std::time::{Duration, Instant};

use dot_transcript::{
    Motion, Selection, Spot, columns, index, paragraph_at, rows, to_f32, word_at,
};

use crate::canvas::Layout;
use crate::conversation::Conversation;
use crate::conversation::stage::Stage;

const EDGE: f32 = 24.0;
const EDGE_ROWS_PER_FRAME: f64 = 0.35;
pub(super) const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
pub(super) const SETTLING: Motion = Motion::landing(Duration::from_millis(450));
pub(super) const COPIED_SHOWN: Motion = Motion::landing(Duration::from_millis(2_400));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selecting {
    pub(crate) selection: Selection,
    pub(crate) made: Option<Instant>,
    pub(crate) copied: Option<Instant>,
}

impl Conversation {
    pub(crate) fn select_press(
        &mut self,
        at: [f32; 2],
        clicks: u8,
        layout: &Layout,
        now: Instant,
    ) -> bool {
        if self.question.is_some() || self.agent_view.is_some() {
            return false;
        }
        let Some(spot) = self.text_spot(at, layout, now) else {
            self.selecting = None;
            return false;
        };
        let (selection, made) = match clicks {
            0 | 1 => (Selection::at(spot), None),
            2 => (word_at(&self.transcript, spot), Some(now)),
            _ => (paragraph_at(&self.transcript, spot.row), Some(now)),
        };
        self.selecting = Some(Selecting {
            selection,
            made,
            copied: None,
        });
        true
    }

    pub(crate) fn select_drag(&mut self, at: [f32; 2], layout: &Layout, now: Instant) {
        if !self.is_dragging() {
            return;
        }
        let spot = self.nearest_spot(at, layout, now);
        if let (Some(selecting), Some(spot)) = (self.selecting.as_mut(), spot) {
            selecting.selection.head = spot;
        }
    }

    pub(crate) fn select_release(&mut self, now: Instant) {
        let Some(selecting) = self
            .selecting
            .as_mut()
            .filter(|selecting| selecting.made.is_none())
        else {
            return;
        };
        if selecting.selection.is_empty() {
            self.selecting = None;
            return;
        }
        selecting.selection = selecting.selection.to_words(&self.transcript);
        selecting.made = Some(now);
    }

    pub(crate) fn clear_selection(&mut self) -> bool {
        self.selecting.take().is_some()
    }

    pub(crate) fn selected_text(&self) -> Option<String> {
        let selecting = self
            .selecting
            .as_ref()
            .filter(|selecting| selecting.made.is_some())?;
        Some(selecting.selection.text(&self.transcript)).filter(|text| !text.is_empty())
    }

    pub(crate) fn mark_copied(&mut self, now: Instant) {
        if let Some(selecting) = self.selecting.as_mut() {
            selecting.copied = Some(now);
        }
    }

    pub(crate) fn is_over_text(&self, at: [f32; 2], layout: &Layout, now: Instant) -> bool {
        self.agent_view.is_none() && self.text_spot(at, layout, now).is_some()
    }

    pub(crate) fn follow_edges(&mut self, layout: &Layout, now: Instant) {
        let Some(pointer) = self.pointer.filter(|_| self.is_dragging()) else {
            return;
        };
        let stage = Stage::of(layout);
        let edge = stage.points(EDGE);
        let rows = if pointer[1] < edge {
            -EDGE_ROWS_PER_FRAME
        } else if pointer[1] > stage.floor - edge {
            EDGE_ROWS_PER_FRAME
        } else {
            return;
        };
        self.reader.scroll_by(rows);
        self.select_drag(pointer, layout, now);
    }

    pub(crate) fn is_dragging(&self) -> bool {
        self.selecting
            .as_ref()
            .is_some_and(|selecting| selecting.made.is_none())
    }

    pub(crate) fn selection_moves(&self, now: Instant) -> bool {
        let Some(selecting) = self.selecting.as_ref() else {
            return false;
        };
        let lasts = |at: Option<Instant>, motion: Motion| {
            at.is_some_and(|at| motion.lasts(now.saturating_duration_since(at)))
        };
        self.is_dragging()
            || lasts(selecting.made, COOLING)
            || lasts(selecting.copied, COPIED_SHOWN)
    }

    pub(super) fn text_spot(
        &self,
        [x, y]: [f32; 2],
        layout: &Layout,
        now: Instant,
    ) -> Option<Spot> {
        let stage = Stage::of(layout);
        let row = stage.row_at(y, self.reader.focus(now))?;
        let shown = self.transcript.row_text(row)?;
        let advance = layout.advance();
        let left = stage.column[0] + f32::from(shown.indent) * advance;
        let length = to_f32(rows(columns(shown.text)));
        let inside = x >= left && x <= left + length * advance;
        inside.then(|| Spot {
            row,
            column: column_at(x, left, advance, shown.text),
        })
    }

    fn nearest_spot(&self, [x, y]: [f32; 2], layout: &Layout, now: Instant) -> Option<Spot> {
        let stage = Stage::of(layout);
        let focus = self.reader.focus(now);
        let clamped = y.clamp(0.0, (stage.floor - 1.0).max(0.0));
        let row = stage.row_at(clamped, focus)?;
        let last = self.transcript.total().checked_sub(1)?;
        let row = row.min(last);
        let advance = layout.advance();
        let Some(shown) = self.transcript.row_text(row) else {
            return Some(Spot { row, column: 0 });
        };
        let left = stage.column[0] + f32::from(shown.indent) * advance;
        Some(Spot {
            row,
            column: column_at(x, left, advance, shown.text),
        })
    }
}

fn column_at(x: f32, left: f32, advance: f32, text: &str) -> usize {
    let length = text.chars().count();
    let column = f64::from(((x - left) / advance).max(0.0)).round();
    index(column).min(length)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/select/mod.rs"]
mod tests;
