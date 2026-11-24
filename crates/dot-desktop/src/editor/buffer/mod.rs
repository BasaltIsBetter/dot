mod history;
mod select;

use std::ops::Range;
use std::time::Instant;

use dot_transcript::{TextWidth, typed};

use crate::conversation::{Draft, Edit, Motion};
use crate::editor::buffer::history::{History, Step};

pub(crate) const LONGEST_EDITED: usize = 1_024 * 1_024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Buffer {
    draft: Draft,
    anchor: Option<usize>,
    history: History,
}

impl Buffer {
    pub(crate) fn new(text: String, width: TextWidth) -> Self {
        Self {
            draft: Draft::from_text(text, width, LONGEST_EDITED),
            anchor: None,
            history: History::default(),
        }
    }

    pub(crate) fn text(&self) -> &str {
        self.draft.text()
    }

    pub(crate) fn rows(&self) -> &[Range<usize>] {
        self.draft.rows()
    }

    pub(crate) fn caret_cell(&self) -> (usize, usize) {
        self.draft.caret_cell()
    }

    pub(crate) fn heat(&self, row: Range<usize>, now: Instant) -> Vec<(Range<usize>, u8)> {
        self.draft.typed().runs(row, now)
    }

    pub(crate) fn rewrap_to(&mut self, width: TextWidth) {
        if self.draft.width() != width {
            let caret = self.draft.caret();
            self.draft.rewrap_to(width);
            self.draft.set_caret(caret);
        }
    }

    pub(crate) fn go(&mut self, motion: Motion, extend: bool, now: Instant) {
        self.hold_anchor(extend);
        self.draft.apply(Edit::Move(motion), now);
    }

    pub(crate) fn place(&mut self, row: usize, column: usize, extend: bool) {
        self.hold_anchor(extend);
        self.draft.place(row, column);
    }

    pub(crate) fn insert(&mut self, text: &str, now: Instant) -> bool {
        let range = self.selection().unwrap_or_else(|| self.caret_range());
        let room = self.draft.room() + self.text()[range.clone()].chars().count();
        let text = typed(text, room);
        if text.is_empty() && range.is_empty() {
            return false;
        }
        self.change(range, &text, now, true);
        true
    }

    pub(crate) fn erase(&mut self, motion: Motion, now: Instant) -> bool {
        let range = self.selection().unwrap_or_else(|| {
            let caret = self.draft.caret();
            let target = self.draft.erase_target(motion);
            caret.min(target)..caret.max(target)
        });
        if range.is_empty() {
            return false;
        }
        self.change(range, "", now, true);
        true
    }

    pub(crate) fn cut(&mut self, now: Instant) -> Option<String> {
        let range = self.selection()?;
        let cut = self.text()[range.clone()].to_owned();
        self.change(range, "", now, false);
        Some(cut)
    }

    pub(crate) fn replace_all(&mut self, text: &str, now: Instant) -> bool {
        if self.text() == text {
            return false;
        }
        let caret = self.draft.caret();
        self.change(0..self.text().len(), text, now, false);
        let kept = (0..=caret.min(text.len()))
            .rev()
            .find(|&offset| text.is_char_boundary(offset))
            .unwrap_or(0);
        self.draft.set_caret(kept);
        true
    }

    pub(crate) fn undo(&mut self, now: Instant) -> bool {
        let Some(step) = self.history.undo() else {
            return false;
        };
        let range = step.at..step.at + step.inserted.len();
        self.draft.replace(range, &step.removed, now);
        self.draft.set_caret(step.caret);
        self.anchor = None;
        true
    }

    pub(crate) fn redo(&mut self, now: Instant) -> bool {
        let Some(step) = self.history.redo() else {
            return false;
        };
        let range = step.at..step.at + step.removed.len();
        self.draft.replace(range, &step.inserted, now);
        self.anchor = None;
        true
    }

    fn change(&mut self, range: Range<usize>, text: &str, now: Instant, joins: bool) {
        let caret = self.draft.caret();
        let removed = self.draft.replace(range.clone(), text, now);
        self.anchor = None;
        let step = Step {
            at: range.start,
            removed,
            inserted: text.to_owned(),
            caret,
            when: now,
        };
        self.history.record(step, joins);
    }

    fn caret_range(&self) -> Range<usize> {
        let caret = self.draft.caret();
        caret..caret
    }

    fn hold_anchor(&mut self, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.draft.caret());
        } else {
            self.anchor = None;
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/buffer/mod.rs"]
mod tests;
