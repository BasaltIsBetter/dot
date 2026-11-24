use std::ops::Range;

use crate::conversation::{line_around, word_around};
use crate::editor::buffer::Buffer;

impl Buffer {
    pub(crate) fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let caret = self.draft.caret();
        (anchor != caret).then(|| anchor.min(caret)..anchor.max(caret))
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.selection().map(|range| &self.text()[range])
    }

    pub(crate) fn select_word(&mut self, row: usize, column: usize) -> Range<usize> {
        self.draft.place(row, column);
        self.select(word_around(self.text(), self.draft.caret()))
    }

    pub(crate) fn select_line(&mut self, row: usize, column: usize) -> Range<usize> {
        self.draft.place(row, column);
        self.select(line_around(self.text(), self.draft.caret()))
    }

    pub(crate) fn fixed_end(&self) -> usize {
        self.anchor.unwrap_or_else(|| self.draft.caret())
    }

    pub(crate) fn drag_to(&mut self, row: usize, column: usize, held: &Range<usize>) {
        self.draft.place(row, column);
        let caret = self.draft.caret();
        if caret < held.start {
            self.anchor = Some(held.end);
        } else if caret > held.end {
            self.anchor = Some(held.start);
        } else {
            self.anchor = Some(held.start);
            self.draft.set_caret(held.end);
        }
    }

    pub(crate) fn select_all(&mut self) {
        self.anchor = Some(0);
        self.draft.set_caret(self.text().len());
    }

    fn select(&mut self, range: Range<usize>) -> Range<usize> {
        self.anchor = Some(range.start);
        self.draft.set_caret(range.end);
        range
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/buffer/select.rs"]
mod tests;
