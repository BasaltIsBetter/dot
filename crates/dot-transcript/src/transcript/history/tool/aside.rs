use std::time::Instant;

use crate::transcript::entry::Entry;
use crate::transcript::history::Transcript;
use crate::transcript::tool::Aside;

impl Transcript {
    pub(super) fn sum_up(&mut self, position: usize, aside: Aside<'_>, now: Instant) {
        let width = self.width;
        let Some(sum) = self.command_sum_beside(position) else {
            let extended = self
                .entries
                .get_mut(position)
                .is_some_and(|entry| entry.sum_up(aside, now, width));
            if extended {
                self.recount_from(position);
                self.advance_ghost(now);
            }
            return;
        };
        self.settled = self.settled.min(sum);
        let Some(entry) = self.entries.get_mut(sum) else {
            return;
        };
        let rows = entry.row_count();
        if !entry.replace_aside(aside, now, width) {
            return;
        }
        if entry.row_count() != rows {
            self.recount_from(sum);
        }
        self.advance_ghost(now);
    }

    fn command_sum_beside(&self, position: usize) -> Option<usize> {
        if !self.entries.get(position)?.runs_a_command() {
            return None;
        }
        [self.visible_before(position), self.visible_after(position)]
            .into_iter()
            .flatten()
            .find(|&beside| {
                self.is_in_reply(beside)
                    && self
                        .entries
                        .get(beside)
                        .is_some_and(Entry::sums_up_a_command)
            })
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-transcript/transcript/history/tool/aside.rs"]
mod tests;
