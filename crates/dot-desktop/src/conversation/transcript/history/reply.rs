use std::time::Instant;

use crate::canvas::{Glide, PIN, SETTLE, to_f32};
use crate::conversation::scroll::rows;
use crate::conversation::transcript::entry::{Entry, Kind};
use crate::conversation::transcript::history::Transcript;

impl Transcript {
    pub(crate) fn begin_reply(&mut self, visible: f64, now: Instant) {
        self.veil_question();
        let waiting = Entry::empty(Kind::Answer, self.width);
        let first_row = rows(self.total() + self.gap_at(&waiting, self.entries.len()));
        self.push(waiting, now);
        self.live = true;
        let height = rows(self.ghost_rows().max(1));
        self.pin_to_top(first_row, height, visible, now);
    }

    pub(crate) fn hear(&mut self, kind: Kind, piece: &str, visible: f64, now: Instant) {
        if !self.live {
            return;
        }
        self.follow_growth(visible, now, |transcript| {
            transcript.write(kind, piece, now);
        });
    }

    pub(crate) fn end_reply(&mut self, notice: Option<&str>, visible: f64, now: Instant) {
        if !self.live {
            return;
        }
        self.follow_growth(visible, now, |transcript| transcript.close(notice, now));
    }

    pub(crate) const fn is_live(&self) -> bool {
        self.live
    }

    fn write(&mut self, kind: Kind, piece: &str, now: Instant) {
        match self.entries.last_mut() {
            Some(last) if last.kind == kind || last.is_empty() => {
                last.kind = kind;
                last.extend(piece, now, self.width);
                self.refresh_last_end();
                self.last_arrival = Some(now);
                self.advance_ghost(now);
            }
            _ => self.push(Entry::new(kind, piece, now, self.width), now),
        }
    }

    fn close(&mut self, notice: Option<&str>, now: Instant) {
        self.live = false;
        self.settle_running_tools(now);
        let silent = self.entries.last().is_some_and(Entry::is_empty);
        if silent {
            self.entries.pop();
            self.ends.pop();
        }

        match notice {
            Some(notice) => {
                self.push(Entry::new(Kind::Notice, notice, now, self.width), now);
                self.advance_ghost(now);
            }
            None if silent => {
                self.drop_ghost();
                self.scroll.unpin();
                self.travel = None;
            }
            None => {}
        }
        self.finish_ghost(now);
    }

    pub(super) fn follow_growth(
        &mut self,
        visible: f64,
        now: Instant,
        change: impl FnOnce(&mut Self),
    ) {
        let before = self.scroll.past_pin(self.total(), visible);
        change(self);
        let after = self.scroll.past_pin(self.total(), visible);

        if let (Some(before), Some(after)) = (before, after)
            && after > before
        {
            self.ease_down(after - before, visible, now);
        }
    }

    fn ease_down(&mut self, rows: f64, visible: f64, now: Instant) {
        let lag = self
            .travel
            .map_or(0.0, |travel| f64::from(travel.position(now)));
        let departure = (lag - rows).max(-visible);

        let mut travel = Glide::at_rest(to_f32(departure), SETTLE);
        travel.toward(0.0, now);
        self.travel = Some(travel);
    }

    fn pin_to_top(&mut self, row: f64, height: f64, visible: f64, now: Instant) {
        let at_the_bottom = row + height - visible;
        let departure = self.shown_top(visible, now).max(at_the_bottom).min(row);

        self.scroll.pin(row);
        let mut travel = Glide::at_rest(to_f32(departure - row), PIN);
        travel.toward(0.0, now);
        self.travel = Some(travel);
    }

    pub(super) fn refresh_last_end(&mut self) {
        let Some(entry) = self.entries.last() else {
            return;
        };
        let last = self.ends.len() - 1;
        let end = self.start_of(last) + self.gap_at(entry, last) + entry.rows.len();
        self.ends[last] = end;
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/history/reply.rs"]
mod tests;
