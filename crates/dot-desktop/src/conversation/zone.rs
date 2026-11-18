use crate::canvas::{Layout, to_f32};
use crate::conversation::scroll::{index, rows};

const TOP_INSET: f32 = 32.0;
const PROMPTER_GAP: f32 = 12.0;
const RULE_GAP: f32 = 12.0;
const PROMPTER_MOST_ROWS: usize = 8;
const PROMPTER_MOST_SHARE: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Band {
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Area {
    Transcript,
    Prompter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Zones {
    pub(crate) transcript: Band,
    pub(crate) rule: f32,
    pub(crate) prompter: Band,
    pub(crate) prompter_rows: usize,
    floor: f32,
}

impl Band {
    pub(crate) fn rows(self, line_height: f32) -> f64 {
        if line_height <= 0.0 {
            return 0.0;
        }
        f64::from(((self.bottom - self.top) / line_height).max(0.0))
    }

    pub(crate) fn row_top(self, rows_down: f64, line_height: f32) -> f32 {
        (self.top + to_f32(rows_down) * line_height).round()
    }
}

impl Zones {
    pub(crate) fn new(layout: &Layout, draft_rows: usize) -> Self {
        let line_height = layout.line_height();
        let floor = layout.strip_top();
        let bottom = floor - layout.scaled(PROMPTER_GAP);
        let top = layout.scaled(TOP_INSET);
        let gap = layout.scaled(RULE_GAP);

        let between = Band {
            top,
            bottom: bottom - 2.0 * gap,
        };
        let most =
            index(between.rows(line_height) * PROMPTER_MOST_SHARE).clamp(1, PROMPTER_MOST_ROWS);
        let shown = draft_rows.clamp(1, most);
        let prompter_top = (bottom - to_f32(rows(shown)) * line_height).round();
        let rule = prompter_top - gap;

        Self {
            transcript: Band {
                top,
                bottom: rule - gap,
            },
            rule,
            prompter: Band {
                top: prompter_top,
                bottom,
            },
            prompter_rows: shown,
            floor,
        }
    }

    pub(crate) fn area_at(&self, y: f32) -> Option<Area> {
        if y < self.rule {
            return Some(Area::Transcript);
        }
        (y < self.floor).then_some(Area::Prompter)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/zone.rs"]
mod tests;
