use std::ops::Range;

use crate::conversation::prompter::motion::{Cursor, Motion, Spot};
use crate::conversation::wrap::{typed, wrap_characters};
use crate::conversation::zone::TextWidth;

const LONGEST: usize = 1_000_000;
const NEW_LINE: char = '\n';

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    Insert(String),
    Move(Motion),
    Erase(Motion),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Draft {
    text: String,
    caret: Spot,
    rows: Vec<Range<usize>>,
    goal: Option<usize>,
    width: TextWidth,
}

impl Default for Draft {
    fn default() -> Self {
        Self::new(TextWidth::default())
    }
}

impl Draft {
    fn new(width: TextWidth) -> Self {
        Self {
            text: String::new(),
            caret: Spot::default(),
            rows: wrap_characters("", width.columns()),
            goal: None,
            width,
        }
    }

    pub(crate) fn apply(&mut self, edit: Edit) {
        match edit {
            Edit::Insert(text) => self.insert(&text),
            Edit::Move(motion) => self.move_caret(motion),
            Edit::Erase(motion) => self.erase(motion),
        }
    }

    pub(crate) fn place(&mut self, row: usize, column: usize) {
        self.caret = self.cursor().spot_at(row, column);
        self.goal = None;
    }

    pub(crate) fn take(&mut self) -> Option<String> {
        let first_visible = self
            .text
            .find(|character: char| !character.is_whitespace())?;
        let start = self.text[..first_visible]
            .rfind(NEW_LINE)
            .map_or(0, |new_line| new_line + NEW_LINE.len_utf8());
        let sent = self.text[start..].trim_end().to_owned();

        self.clear();
        Some(sent)
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::new(self.width);
    }

    pub(crate) fn rewrap_to(&mut self, width: TextWidth) {
        self.width = width;
        self.rewrap();
    }

    pub(crate) const fn width(&self) -> TextWidth {
        self.width
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn rows(&self) -> &[Range<usize>] {
        &self.rows
    }

    pub(crate) fn caret_cell(&self) -> (usize, usize) {
        let cursor = self.cursor();
        (cursor.row(), cursor.column())
    }

    fn insert(&mut self, text: &str) {
        let room = LONGEST.saturating_sub(self.text.chars().count());
        let typed = typed(text, room);
        if typed.is_empty() {
            return;
        }

        self.text.insert_str(self.caret.offset, &typed);
        self.caret = Spot::at(self.caret.offset + typed.len());
        self.rewrap();
    }

    fn move_caret(&mut self, motion: Motion) {
        let cursor = self.cursor();
        let goal = self.goal.unwrap_or_else(|| cursor.column());
        self.caret = cursor.target(motion, goal);
        self.goal = matches!(motion, Motion::Up | Motion::Down).then_some(goal);
    }

    fn erase(&mut self, motion: Motion) {
        let cursor = self.cursor();
        let caret = self.caret.offset;
        let target = match cursor.target(motion, cursor.column()).offset {
            unmoved if unmoved == caret && motion == Motion::RowStart => {
                cursor.target(Motion::Left, 0).offset
            }
            target => target,
        };
        let erased = caret.min(target)..caret.max(target);
        if erased.is_empty() {
            return;
        }

        self.text.replace_range(erased.clone(), "");
        self.caret = Spot::at(erased.start);
        self.rewrap();
    }

    fn rewrap(&mut self) {
        self.rows = wrap_characters(&self.text, self.width.columns());
        self.goal = None;
    }

    fn cursor(&self) -> Cursor<'_> {
        Cursor {
            text: &self.text,
            rows: &self.rows,
            caret: self.caret,
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/draft.rs"]
mod tests;
