#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Scroll {
    first: Option<f64>,
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

    pub(crate) fn window(self, total: usize, visible: f64) -> Window {
        let visible = visible.max(0.0);
        let count = rows(total);
        if count <= visible {
            return Window {
                first: 0,
                end: total,
                offset: visible - count,
            };
        }

        let first = self.first_row(total, visible);
        let start = first.floor();
        Window {
            first: index(start),
            end: index((first + visible).ceil()).min(total),
            offset: start - first,
        }
    }

    fn first_row(self, total: usize, visible: f64) -> f64 {
        let last = last_first_row(total, visible);
        self.first.map_or(last, |first| first.clamp(0.0, last))
    }

    fn settle(&mut self, first: f64, total: usize, visible: f64) {
        let last = last_first_row(total, visible);
        let first = first.clamp(0.0, last);
        self.first = (first < last).then_some(first);
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

fn last_first_row(total: usize, visible: f64) -> f64 {
    (rows(total) - visible.max(0.0)).max(0.0)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/scroll.rs"]
mod tests;
