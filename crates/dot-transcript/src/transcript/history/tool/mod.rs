mod aside;
mod stack;

use std::mem;
use std::time::Instant;

use crate::code::{Change, CodeBlock};
use crate::transcript::entry::Entry;
use crate::transcript::history::{Row, RunningTool, Transcript};
use crate::transcript::tool::{Aside, Running, Tool};

impl Transcript {
    /// Starts a tool Claude called during the reply, under `id`; nothing happens outside a reply.
    ///
    /// The question still in the place of the reply leaves it, every word at once, and comes
    /// back above, as if the reply had already replaced it.
    pub fn start_tool(&mut self, id: String, tool: Tool, now: Instant) {
        if !self.is_live() {
            return;
        }
        let position = self.entries.len();
        self.push(Entry::tool(tool, now), now);
        self.running_tools.push(RunningTool { id, position });
        self.finish_ghost(now);
    }

    /// Ends the running tool `id` with the [`Aside`] that sums it up in the reply, if it has
    /// one; an unknown or ended tool changes nothing.
    ///
    /// A command's aside takes the place of the aside of another command right before or after
    /// it in the reply, with nothing shown between them: commands run one after the other keep
    /// a single row, the last one to end on it, its words that changed rising over 300 ms.
    pub fn end_tool(&mut self, id: &str, aside: Option<Aside<'_>>, now: Instant) {
        let Some(found) = self.running_tools.iter().position(|tool| tool.id == id) else {
            return;
        };
        let position = self.running_tools.remove(found).position;
        let Some(entry) = self.entries.get_mut(position) else {
            return;
        };
        entry.end(now);
        self.last_arrival = Some(now);
        if let Some(aside) = aside {
            self.sum_up(position, aside, now);
        }
    }

    /// Shows under the running tool `id` the `change` it made to a file, line by line: its rows
    /// take their place in the reply, and its lines appear and change for a moment from `now`.
    /// An unknown or ended tool, or a change with nothing to show, changes nothing.
    ///
    /// A block right after other blocks, with nothing shown between them, stacks on them: the
    /// one before it folds to a row with its name and the lines it added and removed, and
    /// every folded name of the stack ends at the same column.
    pub fn change_tool(&mut self, id: &str, change: Change, now: Instant) {
        let Some(position) = self
            .running_tools
            .iter()
            .find(|tool| tool.id == id)
            .map(|tool| tool.position)
        else {
            return;
        };
        let Some(code) = CodeBlock::new(change, now) else {
            return;
        };
        let until = code.lasts_until();
        let Some(entry) = self.entries.get_mut(position) else {
            return;
        };
        entry.show_code(code);
        self.code_until = Some(self.code_until.map_or(until, |earlier| earlier.max(until)));
        let first = self.stack_code(position).unwrap_or(position);
        self.recount_from(first);
        if self.entries[position]
            .code()
            .is_some_and(CodeBlock::is_growing)
        {
            self.growing.push(position);
        }
        self.advance_ghost(now);
    }

    /// Returns when the blocks still being written will have taken all their rows: the moment
    /// the reply can go on below them without pushing anything down; `None` once they all have.
    #[must_use]
    pub fn grows_until(&self) -> Option<Instant> {
        self.growing
            .iter()
            .filter_map(|&position| self.entries.get(position)?.code())
            .filter(|code| code.is_growing())
            .map(CodeBlock::grown_at)
            .max()
    }

    /// Gives every block still being written the rows of the lines written by `now`: a block
    /// grows row by row as its lines appear, and pushes what follows it down as it grows.
    pub fn grow_code(&mut self, now: Instant) {
        let mut first: Option<usize> = None;
        for position in mem::take(&mut self.growing) {
            let Some(entry) = self.entries.get_mut(position) else {
                continue;
            };
            let Some(code) = entry.code_mut() else {
                continue;
            };
            let grew = code.grow(now);
            if code.is_growing() {
                self.growing.push(position);
            }
            if grew {
                entry.recount_code();
                first = Some(first.map_or(position, |first| first.min(position)));
            }
        }
        if let Some(first) = first {
            self.recount_from(first);
        }
    }

    /// Opens or closes what a click at `column` of `row` folds: lines of code, or a block
    /// stacked on others, from past its name; returns whether there was any.
    pub fn toggle_code(&mut self, row: usize, column: usize) -> bool {
        let Some(Row::Text {
            row: within,
            position,
            ..
        }) = self.row(row)
        else {
            return false;
        };
        let Some(entry) = self.entries.get_mut(position) else {
            return false;
        };
        if !entry
            .code_mut()
            .is_some_and(|code| code.toggle(within, column))
        {
            return false;
        }
        entry.recount_code();
        self.recount_from(position);
        true
    }

    /// Returns the file whose name is at `column` of `row` at `now`, as the change showed it,
    /// once it can be opened; on a block that is not stacked, the whole row of its name opens it.
    #[must_use]
    pub fn file_at(&self, row: usize, column: usize, now: Instant) -> Option<&str> {
        let Some(Row::Text {
            entry, row: within, ..
        }) = self.row(row)
        else {
            return None;
        };
        let code = entry.code()?;
        code.opens_file(within, column, now).then(|| code.path())
    }

    /// Keeps `lines`, the last lines the running tool `id` printed, in place of the ones before;
    /// an unknown or ended tool keeps nothing.
    pub fn tool_output(&mut self, id: &str, lines: Vec<String>) {
        let Some(running) = self.running_tools.iter().find(|tool| tool.id == id) else {
            return;
        };
        if let Some(mark) = self
            .entries
            .get_mut(running.position)
            .and_then(|entry| entry.mark.as_deref_mut())
        {
            mark.output = lines;
        }
    }

    /// Returns the tools still running, the first started first, with what they printed last.
    pub fn running(&self) -> impl Iterator<Item = Running<'_>> + '_ {
        self.running_tools.iter().filter_map(|running| {
            let mark = self.entries.get(running.position)?.mark.as_deref()?;
            Some(Running {
                id: &running.id,
                tool: &mark.tool,
                started: mark.started,
                output: &mark.output,
            })
        })
    }

    /// Returns the running tool `id`, as it was started, and when it started.
    #[must_use]
    pub fn running_tool(&self, id: &str) -> Option<(&Tool, Instant)> {
        let running = self.running_tools.iter().find(|tool| tool.id == id)?;
        let mark = self.entries.get(running.position)?.mark.as_deref()?;
        Some((&mark.tool, mark.started))
    }

    /// Returns the tools still running, the first started first.
    pub fn running_tools(&self) -> impl Iterator<Item = &Tool> + '_ {
        self.running_tools.iter().filter_map(|running| {
            let mark = self.entries.get(running.position)?.mark.as_deref()?;
            Some(&mark.tool)
        })
    }

    pub(super) fn settle_running_tools(&mut self, now: Instant) {
        for tool in mem::take(&mut self.running_tools) {
            if let Some(entry) = self.entries.get_mut(tool.position) {
                entry.end(now);
                self.last_arrival = Some(now);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-transcript/transcript/history/tool/mod.rs"]
mod tests;
