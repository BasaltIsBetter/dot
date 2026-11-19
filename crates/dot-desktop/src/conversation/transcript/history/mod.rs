mod reply;
mod rewrap;
mod tool;
mod veil;

use std::time::Instant;

use crate::canvas::Glide;
use crate::conversation::scroll::{Scroll, Window};
use crate::conversation::transcript::entry::{Entry, Kind};
use crate::conversation::transcript::fade::is_fresh;
use crate::conversation::transcript::ghost::Ghost;
use crate::conversation::zone::TextWidth;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
    entries: Vec<Entry>,
    ends: Vec<usize>,
    width: TextWidth,
    scroll: Scroll,
    travel: Option<Glide>,
    ghost: Option<Ghost>,
    live: bool,
    running_tools: Vec<RunningTool>,
    last_arrival: Option<Instant>,
}

#[derive(Debug)]
struct RunningTool {
    id: String,
    position: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row<'a> {
    Gap,
    Text {
        entry: &'a Entry,
        row: usize,
        position: usize,
    },
}

impl Transcript {
    pub(crate) fn append(&mut self, text: &str, now: Instant) {
        self.push(Entry::new(Kind::Prompt, text, now, self.width), now);
        self.scroll.follow();
    }

    pub(crate) fn attach(&mut self, label: &str, now: Instant) {
        self.push(Entry::new(Kind::Attachment, label, now, self.width), now);
    }

    pub(crate) fn notice(&mut self, text: &str, now: Instant) {
        self.push(Entry::new(Kind::Notice, text, now, self.width), now);
        self.scroll.follow();
    }

    pub(crate) fn total(&self) -> usize {
        self.ends.last().copied().unwrap_or_default()
    }

    pub(crate) fn scroll_by(&mut self, rows: f64, visible: f64, now: Instant) {
        self.land(visible, now);
        self.scroll.by(rows, self.total(), visible);
    }

    pub(crate) fn refit(&mut self, visible: f64) {
        self.scroll.refit(self.total(), visible);
    }

    pub(crate) fn window(&self, visible: f64, now: Instant) -> Window {
        Window::at(self.shown_top(visible, now), self.total(), visible)
    }

    pub(crate) fn row(&self, index: usize) -> Option<Row<'_>> {
        let position = self.ends.partition_point(|&end| end <= index);
        let entry = self.entries.get(position)?;
        let local = index - self.start_of(position);
        Some(
            local
                .checked_sub(self.gap_at(entry, position))
                .map_or(Row::Gap, |row| Row::Text {
                    entry,
                    row,
                    position,
                }),
        )
    }

    pub(crate) fn entry(&self, position: usize) -> Option<&Entry> {
        self.entries.get(position)
    }

    pub(crate) fn first_row_of(&self, position: usize) -> Option<usize> {
        let entry = self.entries.get(position)?;
        Some(self.start_of(position) + self.gap_at(entry, position))
    }

    pub(crate) fn is_last_row(&self, index: usize) -> bool {
        index + 1 == self.total()
    }

    pub(crate) fn is_fresh(&self, now: Instant) -> bool {
        self.last_arrival
            .is_some_and(|arrival| is_fresh(now.saturating_duration_since(arrival)))
    }

    pub(crate) fn is_traveling(&self, now: Instant) -> bool {
        self.travel.is_some_and(|travel| travel.is_moving(now))
    }

    pub(crate) fn is_animating(&self, now: Instant) -> bool {
        self.is_fresh(now)
            || self.is_traveling(now)
            || self
                .ghost
                .as_ref()
                .is_some_and(|ghost| ghost.is_rolling(now))
    }

    fn shown_top(&self, visible: f64, now: Instant) -> f64 {
        let lag = self
            .travel
            .map_or(0.0, |travel| f64::from(travel.position(now)));
        self.scroll.top(self.total(), visible) + lag
    }

    fn land(&mut self, visible: f64, now: Instant) {
        if self.is_traveling(now) {
            let shown = self.shown_top(visible, now);
            self.scroll.scroll_to(shown, self.total(), visible);
        }
        self.travel = None;
    }

    fn push(&mut self, entry: Entry, now: Instant) {
        let position = self.entries.len();
        if self
            .ghost
            .as_ref()
            .is_some_and(|ghost| position > ghost.slot())
        {
            self.finish_ghost(now);
        }
        let end = self.total() + self.rows_taken(&entry, position);
        if entry.last_arrival().is_some() {
            self.last_arrival = Some(now);
        }
        self.entries.push(entry);
        self.ends.push(end);
    }

    fn rows_taken(&self, entry: &Entry, position: usize) -> usize {
        if self
            .ghost
            .as_ref()
            .is_some_and(|ghost| ghost.hides(position))
        {
            return 0;
        }
        self.gap_at(entry, position) + entry.rows.len()
    }

    fn recount_from(&mut self, from: usize) {
        self.ends.truncate(from);
        let mut end = self.total();
        for position in from..self.entries.len() {
            end += self.rows_taken(&self.entries[position], position);
            self.ends.push(end);
        }
    }

    fn start_of(&self, position: usize) -> usize {
        position
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous])
    }

    fn gap_at(&self, entry: &Entry, position: usize) -> usize {
        let previous = position
            .checked_sub(1)
            .and_then(|previous| self.entries.get(previous));
        gap_before(previous, entry)
    }
}

fn gap_before(previous: Option<&Entry>, entry: &Entry) -> usize {
    let Some(previous) = previous else {
        return 0;
    };
    let follows_a_tool = entry.kind == Kind::Tool && previous.kind == Kind::Tool;
    usize::from(entry.kind != Kind::Attachment && !follows_a_tool)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/history/mod.rs"]
mod tests;
