use crate::conversation::scroll::{index, rows};
use crate::conversation::transcript::history::Transcript;
use crate::conversation::zone::TextWidth;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Anchor {
    entry: usize,
    offset: Option<usize>,
    fraction: f64,
}

impl Transcript {
    pub(crate) fn rewrap(&mut self, width: TextWidth, visible: f64) {
        let anchor = self.scroll.first().and_then(|first| self.anchor_at(first));
        let pinned = self
            .scroll
            .pinned()
            .and_then(|pinned| self.anchor_at(pinned));
        self.width = width;
        for entry in &mut self.entries {
            entry.rewrap(width);
        }
        self.rebase_ghost();
        self.recount_from(0);
        self.travel = None;

        if let Some(pinned) = pinned {
            self.scroll.move_pin(self.row_of(pinned));
        }
        match anchor {
            Some(anchor) => self
                .scroll
                .scroll_to(self.row_of(anchor), self.total(), visible),
            None => self.refit(visible),
        }
    }

    fn anchor_at(&self, first: f64) -> Option<Anchor> {
        let top = first.floor();
        let top_row = index(top);
        let position = self.ends.partition_point(|&end| end <= top_row);
        let entry = self.entries.get(position)?;
        let row = (top_row - self.start_of(position)).checked_sub(self.gap_at(entry, position));
        Some(Anchor {
            entry: position,
            offset: row.map(|row| entry.row(row).start),
            fraction: first - top,
        })
    }

    fn row_of(&self, anchor: Anchor) -> f64 {
        let entry = &self.entries[anchor.entry];
        let row = anchor.offset.map_or(0, |offset| {
            self.gap_at(entry, anchor.entry) + entry.row_holding(offset)
        });
        rows(self.start_of(anchor.entry) + row) + anchor.fraction
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/history/rewrap.rs"]
mod tests;
