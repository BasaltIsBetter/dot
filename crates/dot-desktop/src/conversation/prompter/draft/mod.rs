mod motion;
mod typed;

use std::ops::Range;
use std::time::Instant;

use crate::conversation::prompter::draft::motion::{Cursor, Spot};

pub(crate) use self::motion::{Motion, line_around, word_around};
use crate::conversation::prompter::draft::typed::Typed;
use dot_transcript::TextWidth;
use dot_transcript::{NEW_LINE, typed, wrap_characters};

pub(crate) const LONGEST: usize = 1_000_000;

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
    typed: Typed,
    characters: usize,
    longest: usize,
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
            typed: Typed::default(),
            characters: 0,
            longest: LONGEST,
        }
    }

    pub(crate) fn from_text(text: String, width: TextWidth, longest: usize) -> Self {
        let mut draft = Self::new(width);
        draft.characters = text.chars().count();
        draft.longest = longest;
        draft.text = text;
        draft.rewrap();
        draft
    }

    pub(crate) fn apply(&mut self, edit: Edit, now: Instant) {
        match edit {
            Edit::Insert(text) => self.insert(&text, now),
            Edit::Move(motion) => self.move_caret(motion),
            Edit::Erase(motion) => self.erase(motion, now),
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
        *self = Self {
            longest: self.longest,
            ..Self::new(self.width)
        };
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

    pub(crate) const fn caret(&self) -> usize {
        self.caret.offset
    }

    pub(crate) fn set_caret(&mut self, offset: usize) {
        self.caret = Spot::at(offset.min(self.text.len()));
        self.goal = None;
    }

    pub(crate) const fn room(&self) -> usize {
        self.longest.saturating_sub(self.characters)
    }

    pub(crate) fn replace(&mut self, range: Range<usize>, text: &str, now: Instant) -> String {
        let removed = self.text[range.clone()].to_owned();
        self.characters = self.characters - removed.chars().count() + text.chars().count();
        self.text.replace_range(range.clone(), text);
        if !range.is_empty() {
            self.typed.erased(&range, now);
        }
        if !text.is_empty() {
            self.typed.inserted(range.start, text.len(), now);
        }
        self.caret = Spot::at(range.start + text.len());
        self.rewrap();
        removed
    }

    pub(crate) fn erase_target(&self, motion: Motion) -> usize {
        let cursor = self.cursor();
        match cursor.target(motion, cursor.column()).offset {
            unmoved if unmoved == self.caret.offset && motion == Motion::RowStart => {
                cursor.target(Motion::Left, 0).offset
            }
            target => target,
        }
    }

    pub(crate) fn caret_cell(&self) -> (usize, usize) {
        let cursor = self.cursor();
        (cursor.row(), cursor.column())
    }

    pub(crate) const fn typed(&self) -> &Typed {
        &self.typed
    }

    fn insert(&mut self, text: &str, now: Instant) {
        let typed = typed(text, self.room());
        if typed.is_empty() {
            return;
        }

        self.characters += typed.chars().count();

        self.text.insert_str(self.caret.offset, &typed);
        self.typed.inserted(self.caret.offset, typed.len(), now);
        self.caret = Spot::at(self.caret.offset + typed.len());
        self.rewrap();
    }

    fn move_caret(&mut self, motion: Motion) {
        let cursor = self.cursor();
        let goal = self.goal.unwrap_or_else(|| cursor.column());
        self.caret = cursor.target(motion, goal);
        self.goal = matches!(motion, Motion::Up | Motion::Down).then_some(goal);
    }

    fn erase(&mut self, motion: Motion, now: Instant) {
        let caret = self.caret.offset;
        let target = self.erase_target(motion);
        let erased = caret.min(target)..caret.max(target);
        if erased.is_empty() {
            return;
        }

        self.characters -= self.text[erased.clone()].chars().count();
        self.text.replace_range(erased.clone(), "");
        self.typed.erased(&erased, now);
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
#[path = "../../../../../../tests/dot-desktop/conversation/prompter/draft/mod.rs"]
mod tests;
