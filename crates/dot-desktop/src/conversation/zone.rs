use crate::canvas::{Layout, to_f32};
use crate::conversation::TEXT_INDENT;
use crate::conversation::scroll::{index, rows};

const PROMPTER_MOST_ROWS: usize = 8;
const PROMPTER_MOST_SHARE: f64 = 0.5;
const FIELD_STEP: usize = 16;
const REGULAR: Spacing = Spacing {
    around_transcript: 64.0,
    above_strip: 32.0,
};
const COMPACT: Spacing = Spacing {
    around_transcript: 32.0,
    above_strip: 16.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWidth(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Density {
    Regular,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Spacing {
    around_transcript: f32,
    above_strip: f32,
}

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
    pub(crate) prompter: Band,
    pub(crate) prompter_rows: usize,
    floor: f32,
}

impl TextWidth {
    pub(crate) fn of(layout: &Layout) -> Self {
        Self(usize::from(layout.columns().saturating_sub(TEXT_INDENT)))
    }

    pub(crate) const fn columns(self) -> usize {
        self.0
    }
}

impl Default for TextWidth {
    fn default() -> Self {
        Self(usize::from(Layout::WIDEST_COLUMN - TEXT_INDENT))
    }
}

impl Density {
    fn of(layout: &Layout) -> Self {
        let holds_the_column = layout.columns() >= Layout::WIDEST_COLUMN;
        if holds_the_column && is_tall_enough(layout) {
            return Self::Regular;
        }
        Self::Compact
    }

    const fn spacing(self) -> Spacing {
        match self {
            Self::Regular => REGULAR,
            Self::Compact => COMPACT,
        }
    }
}

impl Spacing {
    const fn stacked(self) -> f32 {
        2.0 * self.around_transcript + self.above_strip
    }
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
        let spacing = Density::of(layout).spacing();
        let line_height = layout.line_height();
        let floor = layout.strip_top();
        let bottom = floor - layout.scaled(spacing.above_strip);
        let margin = layout.scaled(spacing.around_transcript);

        let between = Band {
            top: margin,
            bottom: bottom - margin,
        };
        let most =
            index(between.rows(line_height) * PROMPTER_MOST_SHARE).clamp(1, PROMPTER_MOST_ROWS);
        let shown = draft_rows.clamp(1, most);
        let prompter_top = (bottom - to_f32(rows(shown)) * line_height).round();

        Self {
            transcript: Band {
                top: margin,
                bottom: prompter_top - margin,
            },
            prompter: Band {
                top: prompter_top,
                bottom,
            },
            prompter_rows: shown,
            floor,
        }
    }

    pub(crate) fn area_at(&self, y: f32) -> Option<Area> {
        if y < self.prompter.top {
            return Some(Area::Transcript);
        }
        (y < self.floor).then_some(Area::Prompter)
    }
}

pub(crate) fn centring(layout: &Layout, content_columns: Option<usize>) -> f32 {
    let Some(content) = content_columns else {
        return 0.0;
    };
    let columns = usize::from(layout.columns());
    let field = content.next_multiple_of(FIELD_STEP).min(columns);
    let spare = columns.saturating_sub(field);
    f32::from(u16::try_from(spare).unwrap_or(u16::MAX)) / 2.0
}

fn is_tall_enough(layout: &Layout) -> bool {
    let room = layout.strip_top() - layout.scaled(REGULAR.stacked());
    let grown_prompter_and_as_much_above = rows(2 * PROMPTER_MOST_ROWS);
    room >= to_f32(grown_prompter_and_as_much_above) * layout.line_height()
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/zone.rs"]
mod tests;
