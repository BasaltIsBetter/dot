#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Scroll {
    first: Option<f64>,
    pinned: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Window {
    pub(crate) first: usize,
    pub(crate) end: usize,
    pub(crate) offset: f64,
}

impl Scroll {
    pub(crate) fn by(&mut self, rows: f64, total: usize, visible: f64) {
        if !rows.is_finite() {
            return;
        }
        self.settle(self.first_row(total, visible) - rows, total, visible);
    }

    pub(crate) fn reveal(&mut self, row: usize, total: usize, visible: f64) {
        let first = self.first_row(total, visible);
        let top = rows(row);
        if top < first {
            self.settle(top, total, visible);
        } else if top + 1.0 > first + visible {
            self.settle(top + 1.0 - visible, total, visible);
        }
    }

    pub(crate) fn refit(&mut self, total: usize, visible: f64) {
        if let Some(first) = self.first {
            self.settle(first, total, visible);
        }
    }

    pub(crate) const fn follow(&mut self) {
        self.first = None;
    }

    pub(crate) const fn first(self) -> Option<f64> {
        self.first
    }

    pub(crate) fn scroll_to(&mut self, first: f64, total: usize, visible: f64) {
        self.settle(first, total, visible);
    }

    pub(crate) const fn pin(&mut self, row: f64) {
        self.pinned = Some(row);
        self.first = None;
    }

    pub(crate) const fn pinned(self) -> Option<f64> {
        self.pinned
    }

    pub(crate) fn past_pin(self, total: usize, visible: f64) -> Option<f64> {
        self.pinned.map(|pinned| self.top(total, visible) - pinned)
    }

    pub(crate) const fn move_pin(&mut self, row: f64) {
        self.pinned = Some(row);
    }

    pub(crate) fn shift(&mut self, from: f64, rows: f64) {
        let moved = |row: f64| if row >= from { row + rows } else { row };
        self.first = self.first.map(moved);
        self.pinned = self.pinned.map(moved);
    }

    pub(crate) fn unpin(&mut self) {
        let Some(pinned) = self.pinned.take() else {
            return;
        };
        if self.first.is_some_and(|first| first >= pinned) {
            self.first = None;
        }
    }

    pub(crate) fn top(self, total: usize, visible: f64) -> f64 {
        let bottom = self.bottom(total, visible);
        if bottom <= 0.0 {
            return bottom;
        }
        self.first.map_or(bottom, |first| first.clamp(0.0, bottom))
    }

    pub(crate) fn window(self, total: usize, visible: f64) -> Window {
        Window::at(self.top(total, visible), total, visible)
    }

    fn first_row(self, total: usize, visible: f64) -> f64 {
        self.top(total, visible).max(0.0)
    }

    fn settle(&mut self, first: f64, total: usize, visible: f64) {
        let bottom = self.bottom(total, visible).max(0.0);
        let first = first.clamp(0.0, bottom);
        self.first = (first < bottom).then_some(first);
    }

    fn bottom(self, total: usize, visible: f64) -> f64 {
        let end = end_top(total, visible);
        self.pinned.map_or(end, |pinned| end.max(pinned))
    }
}

impl Window {
    pub(crate) fn at(top: f64, total: usize, visible: f64) -> Self {
        let start = top.floor().max(0.0);
        let end = index((top + visible.max(0.0)).ceil()).min(total);
        Self {
            first: index(start).min(end),
            end,
            offset: start - top,
        }
    }
}

pub(crate) fn rows(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the row is floored at zero first, and a float cast to usize saturates"
)]
pub(crate) fn index(row: f64) -> usize {
    row.max(0.0) as usize
}

fn end_top(total: usize, visible: f64) -> f64 {
    rows(total) - visible.max(0.0)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/scroll.rs"]
mod tests;
