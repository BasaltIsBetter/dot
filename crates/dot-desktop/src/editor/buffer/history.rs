use std::collections::VecDeque;
use std::time::{Duration, Instant};

use dot_transcript::NEW_LINE;

const TYPING_PAUSE: Duration = Duration::from_millis(1_000);
const MOST_STEPS: usize = 500;
const MOST_UNDO_BYTES: usize = 16 * 1_024 * 1_024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Step {
    pub(super) at: usize,
    pub(super) removed: String,
    pub(super) inserted: String,
    pub(super) caret: usize,
    pub(super) when: Instant,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct History {
    undone: VecDeque<Step>,
    redone: Vec<Step>,
    bytes: usize,
}

impl History {
    pub(super) fn record(&mut self, step: Step, joins: bool) {
        self.forget_redone();
        self.bytes = self.bytes.saturating_add(step.bytes());
        if joins && let Some(last) = self.undone.back_mut().filter(|last| last.joins(&step)) {
            last.absorb(&step);
        } else {
            self.undone.push_back(step);
        }
        self.forget_the_oldest();
    }

    pub(super) fn undo(&mut self) -> Option<&Step> {
        let step = self.undone.pop_back()?;
        self.redone.push(step);
        self.redone.last()
    }

    pub(super) fn redo(&mut self) -> Option<&Step> {
        let step = self.redone.pop()?;
        self.undone.push_back(step);
        self.undone.back()
    }

    fn forget_redone(&mut self) {
        for step in self.redone.drain(..) {
            self.bytes = self.bytes.saturating_sub(step.bytes());
        }
    }

    fn forget_the_oldest(&mut self) {
        while self.undone.len() > MOST_STEPS
            || (self.bytes > MOST_UNDO_BYTES && self.undone.len() > 1)
        {
            let Some(oldest) = self.undone.pop_front() else {
                return;
            };
            self.bytes = self.bytes.saturating_sub(oldest.bytes());
        }
    }
}

impl Step {
    fn bytes(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }

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
#[path = "../../../../../tests/dot-desktop/editor/buffer/history.rs"]
mod tests;
