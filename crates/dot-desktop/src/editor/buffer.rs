use std::ops::Range;
use std::time::{Duration, Instant};

use dot_transcript::{NEW_LINE, TextWidth, typed};

use crate::conversation::{Draft, Edit, Motion};

const TYPING_PAUSE: Duration = Duration::from_millis(1_000);
const MOST_STEPS: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Step {
    at: usize,
    removed: String,
    inserted: String,
    caret: usize,
    when: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Buffer {
    draft: Draft,
    anchor: Option<usize>,
    undone: Vec<Step>,
    redone: Vec<Step>,
}

impl Buffer {
    pub(crate) fn new(text: String, width: TextWidth) -> Self {
        Self {
            draft: Draft::from_text(text, width),
            anchor: None,
            undone: Vec::new(),
            redone: Vec::new(),
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

    pub(crate) fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let caret = self.draft.caret();
        (anchor != caret).then(|| anchor.min(caret)..anchor.max(caret))
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.selection().map(|range| &self.text()[range])
    }

    pub(crate) fn go(&mut self, motion: Motion, extend: bool, now: Instant) {
        self.hold_anchor(extend);
        self.draft.apply(Edit::Move(motion), now);
    }

    pub(crate) fn place(&mut self, row: usize, column: usize, extend: bool) {
        self.hold_anchor(extend);
        self.draft.place(row, column);
    }

    pub(crate) fn select_all(&mut self) {
        self.anchor = Some(0);
        self.draft.set_caret(self.text().len());
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
        let Some(step) = self.undone.pop() else {
            return false;
        };
        let range = step.at..step.at + step.inserted.len();
        self.draft.replace(range, &step.removed, now);
        self.draft.set_caret(step.caret);
        self.anchor = None;
        self.redone.push(step);
        true
    }

    pub(crate) fn redo(&mut self, now: Instant) -> bool {
        let Some(step) = self.redone.pop() else {
            return false;
        };
        let range = step.at..step.at + step.removed.len();
        self.draft.replace(range, &step.inserted, now);
        self.anchor = None;
        self.undone.push(step);
        true
    }

    fn change(&mut self, range: Range<usize>, text: &str, now: Instant, joins: bool) {
        let caret = self.draft.caret();
        let removed = self.draft.replace(range.clone(), text, now);
        self.anchor = None;
        self.redone.clear();
        let step = Step {
            at: range.start,
            removed,
            inserted: text.to_owned(),
            caret,
            when: now,
        };
        if joins && let Some(last) = self.undone.last_mut().filter(|last| last.joins(&step)) {
            last.absorb(&step);
            return;
        }
        self.undone.push(step);
        if self.undone.len() > MOST_STEPS {
            self.undone.remove(0);
        }
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

impl Step {
    fn joins(&self, next: &Self) -> bool {
        let soon = next.when.saturating_duration_since(self.when) < TYPING_PAUSE;
        let typing = self.removed.is_empty()
            && next.removed.is_empty()
            && !self.inserted.contains(NEW_LINE)
            && !next.inserted.contains(NEW_LINE)
            && next.at == self.at + self.inserted.len();
        let erasing = self.inserted.is_empty()
            && next.inserted.is_empty()
            && (next.at + next.removed.len() == self.at || next.at == self.at);
        soon && (typing || erasing)
    }

    fn absorb(&mut self, next: &Self) {
        if next.removed.is_empty() {
            self.inserted.push_str(&next.inserted);
        } else if next.at == self.at {
            self.removed.push_str(&next.removed);
        } else {
            self.at = next.at;
            self.removed.insert_str(0, &next.removed);
        }
        self.when = next.when;
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/editor/buffer.rs"]
mod tests;
