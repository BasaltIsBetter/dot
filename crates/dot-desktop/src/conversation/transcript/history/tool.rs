use std::mem;
use std::time::Instant;

use crate::conversation::transcript::entry::{Entry, Kind};
use crate::conversation::transcript::history::{Reply, RunningTool, Transcript};

impl Transcript {
    pub(crate) fn start_tool(&mut self, id: String, shown: &str, visible: f64, now: Instant) {
        if !self.is_live() {
            return;
        }
        self.follow_growth(visible, now, |transcript| {
            transcript.open_tool(id, shown, now);
        });
    }

    pub(crate) fn end_tool(&mut self, id: &str, detail: &str, visible: f64, now: Instant) {
        let Some(found) = self.running_tools.iter().position(|tool| tool.id == id) else {
            return;
        };
        let position = self.running_tools.remove(found).position;
        self.follow_growth(visible, now, |transcript| {
            transcript.settle_tool(position, detail, now);
        });
    }

    pub(super) fn settle_running_tools(&mut self, now: Instant) {
        for tool in mem::take(&mut self.running_tools) {
            if let Some(entry) = self.entries.get_mut(tool.position) {
                entry.ended = Some(now);
                self.last_arrival = Some(now);
            }
        }
    }

    fn open_tool(&mut self, id: String, shown: &str, now: Instant) {
        match self.entries.last_mut() {
            Some(waiting) if self.reply == Reply::Waiting => {
                waiting.kind = Kind::Tool;
                waiting.extend(shown, now, self.width);
                self.refresh_last_end();
                self.last_arrival = Some(now);
                self.advance_ghost(now);
            }
            _ => self.push(Entry::new(Kind::Tool, shown, now, self.width), now),
        }
        self.reply = Reply::Writing;
        let position = self.entries.len() - 1;
        self.running_tools.push(RunningTool { id, position });
    }

    fn settle_tool(&mut self, position: usize, detail: &str, now: Instant) {
        let width = self.width;
        let Some(entry) = self.entries.get_mut(position) else {
            return;
        };
        entry.ended = Some(now);
        entry.extend(detail, now, width);
        self.last_arrival = Some(now);
        self.recount_from(position);
        if self.ghost().is_some_and(|ghost| ghost.slot() == position) {
            self.advance_ghost(now);
        }
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/history/tool.rs"]
mod tests;
