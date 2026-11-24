use std::time::Instant;

use crate::transcript::entry::{Entry, Kind};
use crate::transcript::history::{Reply, Transcript};

impl Transcript {
    /// Starts Claude's reply to the last prompt: the prompt is veiled and the reply will take its
    /// place, word by word, from the row the prompt started on.
    pub fn begin_reply(&mut self, now: Instant) {
        self.settle_cold(now);
        self.veil_question();
        self.reply = Reply::Waiting;
        self.reply_start = self.entries.len();
    }

    /// Adds a `piece` of the reply, thinking or answer as `kind` says; nothing outside a reply.
    pub fn hear(&mut self, kind: Kind, piece: &str, now: Instant) {
        if !self.is_live() {
            return;
        }
        if let Some(ghost) = self.ghost.as_mut() {
            ghost.forget_how_it_left(now);
        }
        self.write(kind, piece, now);
    }

    /// Ends the reply, with the `notice` that says why when it went wrong or was stopped.
    pub fn end_reply(&mut self, notice: Option<&str>, now: Instant) {
        if !self.is_live() {
            return;
        }
        let silent = self.reply == Reply::Waiting && !self.has_reply_rows();
        self.reply = Reply::Idle;
        self.settle_running_tools(now);

        match notice {
            Some(notice) => {
                self.push(Entry::new(Kind::Notice, notice, now, self.width), now);
                self.advance_ghost(now);
            }
            None if silent => self.drop_ghost(),
            None => {}
        }
        self.finish_ghost(now);
        for entry in self.entries.iter_mut().skip(self.reply_start) {
            entry.give_back_room();
        }
    }

    /// Returns whether Claude is replying.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        !matches!(self.reply, Reply::Idle)
    }

    /// Returns the row the reply starts on, while Claude replies: where the prompt was typed.
    #[must_use]
    pub fn anchor(&self) -> Option<usize> {
        if !self.is_live() {
            return None;
        }
        let start = self.start_of(self.reply_start);
        Some(start + usize::from(start > 0))
    }

    /// Returns the row Claude writes on while it replies: the last row of the reply, or the row
    /// it starts on before anything is written.
    #[must_use]
    pub fn writing_row(&self) -> Option<usize> {
        let anchor = self.anchor()?;
        Some(if self.has_reply_rows() {
            self.total() - 1
        } else {
            anchor
        })
    }

    pub(super) fn has_reply_rows(&self) -> bool {
        self.total() > self.start_of(self.reply_start)
    }

    fn write(&mut self, kind: Kind, piece: &str, now: Instant) {
        let position = self.entries.len();
        let extends = position > self.reply_start
            && self.entries[position - 1].kind == kind
            && kind != Kind::Tool;
        if !extends {
            self.push(Entry::new(kind, piece, now, self.width), now);
            self.wrote(now);
            return;
        }
        let width = self.width;
        if self.entries[position - 1].extend(piece, now, width) {
            self.last_arrival = Some(now);
            self.recount_from(position - 1);
            self.wrote(now);
        }
    }

    fn wrote(&mut self, now: Instant) {
        self.reply = Reply::Writing;
        self.advance_ghost(now);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/history/reply.rs"]
mod tests;
