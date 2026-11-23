use std::ops::Range;
use std::time::Instant;

use crate::line::columns;
use crate::reading::rows;
use crate::transcript::entry::{Entry, Kind};
use crate::transcript::ghost::{Cell, Ghost};
use crate::transcript::history::{Insertion, Returned, Row, Transcript};

impl Transcript {
    pub(crate) const fn ghost(&self) -> Option<&Ghost> {
        self.ghost.as_ref()
    }

    pub(crate) fn shows_caret(&self, now: Instant) -> bool {
        self.is_live()
            && self.running_tools.is_empty()
            && self
                .ghost
                .as_ref()
                .is_none_or(|ghost| ghost.is_settled(now))
    }

    pub(crate) const fn is_in_reply(&self, position: usize) -> bool {
        position >= self.reply_start
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
        let Some(cursor) = self.reply_cursor() else {
            return;
        };
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        let was_complete = ghost.is_complete();
        ghost.advance(cursor, now);
        if !was_complete && ghost.is_complete() {
            self.unveil(now);
        }
    }

    pub(super) fn finish_ghost(&mut self, now: Instant) {
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        if ghost.is_complete() {
            return;
        }
        ghost.finish(now);
        self.unveil(now);
    }

    pub(super) fn drop_ghost(&mut self) {
        if let Some(ghost) = self.ghost.take() {
            self.recount_from(ghost.question().start);
        }
    }

    pub(super) fn rebase_ghost(&mut self, now: Instant) {
        let cursor = self.reply_cursor().unwrap_or_default();
        let Some(ghost) = self.ghost.as_mut() else {
            return;
        };
        ghost.rebase(&self.entries, cursor, now);
        let question = ghost.question();
        self.recount_from(question.start);
    }

    fn reply_cursor(&self) -> Option<Cell> {
        let anchor = self.anchor_row()?;
        let last = self.total().checked_sub(1).filter(|&last| last >= anchor)?;
        let Some(Row::Text { entry, row, .. }) = self.row(last) else {
            return None;
        };
        Some((last - anchor, columns(&entry.text[entry.row(row)])))
    }

    fn anchor_row(&self) -> Option<usize> {
        let start = self.start_of(self.reply_start);
        self.ghost.as_ref()?;
        Some(start + usize::from(start > 0))
    }

    fn unveil(&mut self, now: Instant) {
        let Some(question) = self.ghost.as_ref().map(Ghost::question) else {
            return;
        };
        let at = self.anchor_row().unwrap_or_else(|| self.total());
        let before = self.total();
        self.recount_from(question.start);
        let added = self.total().saturating_sub(before);
        self.insertions.push(Insertion {
            at: rows(at),
            rows: rows(added),
        });
        self.returned = Some(Returned { question, at: now });
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
#[path = "../../../../../tests/dot-transcript/transcript/history/veil.rs"]
mod tests;
