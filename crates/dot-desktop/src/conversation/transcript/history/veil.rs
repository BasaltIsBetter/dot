use std::ops::Range;
use std::time::Instant;

use crate::conversation::scroll::rows;
use crate::conversation::transcript::entry::{Entry, Kind};
use crate::conversation::transcript::ghost::Ghost;
use crate::conversation::transcript::history::Transcript;

impl Transcript {
    pub(crate) const fn ghost(&self) -> Option<&Ghost> {
        self.ghost.as_ref()
    }

    pub(super) fn ghost_rows(&self) -> usize {
        self.ghost.as_ref().map_or(0, |ghost| {
            self.entries[ghost.question()]
                .iter()
                .map(Entry::row_count)
                .sum()
        })
    }

    pub(super) fn veil_question(&mut self) {
        self.ghost = self
            .question_at_the_end()
            .map(|question| Ghost::new(question, &self.entries));
        if let Some(question) = self.ghost.as_ref().map(Ghost::question) {
            self.recount_from(question.start);
        }
    }

    pub(super) fn advance_ghost(&mut self, now: Instant) {
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        let Some(slot) = self.entries.get(ghost.slot()) else {
            return;
        };
        let was_complete = ghost.is_complete();
        ghost.advance(slot.cursor(), slot.text.len(), now);
        if !was_complete && ghost.is_complete() {
            self.unveil();
        }
    }

    pub(super) fn finish_ghost(&mut self, now: Instant) {
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        if ghost.is_complete() {
            return;
        }
        let written = self
            .entries
            .get(ghost.slot())
            .map_or(0, |slot| slot.text.len());
        ghost.finish(written, now);
        self.unveil();
    }

    pub(super) fn drop_ghost(&mut self) {
        if let Some(ghost) = self.ghost.take() {
            self.recount_from(ghost.question().start);
        }
    }

    pub(super) fn rebase_ghost(&mut self) {
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        let (cursor, written) = self
            .entries
            .get(ghost.slot())
            .map_or(((0, 0), 0), |slot| (slot.cursor(), slot.text.len()));
        ghost.rebase(&self.entries, cursor, written);
    }

    fn unveil(&mut self) {
        let Some(question) = self.ghost.as_ref().map(Ghost::question) else {
            return;
        };
        let from = rows(self.start_of(question.start));
        let before = self.total();
        self.recount_from(question.start);
        self.scroll.shift(from, rows(self.total() - before));
    }

    fn question_at_the_end(&self) -> Option<Range<usize>> {
        let attachments = self
            .entries
            .iter()
            .rev()
            .take_while(|entry| entry.kind == Kind::Attachment)
            .count();
        let prompt = self.entries.len().checked_sub(attachments + 1)?;
        (self.entries[prompt].kind == Kind::Prompt).then_some(prompt..self.entries.len())
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/history/veil.rs"]
mod tests;
