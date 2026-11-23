mod build;
mod create;
mod edit;
mod ink;

use std::borrow::Cow;
use std::ops::Range;
use std::time::{Duration, Instant};

use crate::code::create::Created;
use crate::code::edit::Edit;
use crate::code::ink::inked;

pub use crate::code::ink::Language as CodeLanguage;
use crate::ink::Ink;
use crate::line::{Line, Span};

/// Lines of a change the conversation keeps: the ones after are counted, not shown.
pub const CODE_LINES_KEPT: usize = 400;

const CONTEXT_SHARE: f64 = 0.4;
const MARK_WIDTH: usize = 3;
const NO_MARK: &str = "   ";
const BAR: &str = " │ ";
const DOT: &str = "·";
const MOST_DOTS: usize = 2;
const FOLDED: &str = "··· ";
const FOLDER_MARK: char = '/';

/// A change a tool made to a file, as the conversation shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// The file, as shown: relative to the session's folder when it lies inside it.
    pub path: String,
    /// Whether the change created the file.
    pub created: bool,
    /// The parts that changed, first to last, each with unchanged lines around it.
    pub hunks: Vec<ChangeHunk>,
    /// Where a created file sits among the entries of its folder, when Dot could tell.
    pub neighbours: Option<Neighbours>,
}

/// A part of a file that changed, with the unchanged lines around it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeHunk {
    /// Number of the hunk's first line in the file after the change, from 1.
    pub new_start: u64,
    /// The lines, in the order of the file.
    pub lines: Vec<ChangeLine>,
}

/// A line of a [`ChangeHunk`], without its line break.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeLine {
    /// A line the change left as it was.
    Same(String),
    /// A line the change took away.
    Removed(String),
    /// A line the change added.
    Added(String),
}

/// The entries listed around a created file in its folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Neighbours {
    /// The folder, as shown, ending with `/`.
    pub folder: String,
    /// The entries listed just before the file, two at most.
    pub before: Vec<String>,
    /// The entries listed just after the file, two at most.
    pub after: Vec<String>,
}

/// The words a code block writes around its numbers, in the language of the interface.
pub trait CodeWords {
    /// Returns what follows a count of lines, such as `lines` after 3 and `line` after 1.
    fn lines(&self, count: u64) -> Cow<'_, str>;
    /// Returns the word for a file Claude just created, such as `new`.
    fn new_file(&self) -> Cow<'_, str>;
    /// Returns what folds the lines of a created file back, such as `fold`.
    fn fold_back(&self) -> Cow<'_, str>;
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drawn {
    Still(Line),
    Rising(Line, f64),
    Leaving(Line, f64),
}

#[derive(Clone, Copy)]
pub(crate) struct Sight<'a> {
    pub(crate) now: Instant,
    pub(crate) hovered: bool,
    pub(crate) room: usize,
    pub(crate) words: &'a dyn CodeWords,
}

struct Look<'a> {
    elapsed: Duration,
    hovered: bool,
    room: usize,
    words: &'a dyn CodeWords,
    name: &'a str,
    language: CodeLanguage,
    open: &'a [bool],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodeBlock {
    path: String,
    language: CodeLanguage,
    started: Instant,
    body: Body,
    open: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Body {
    Edit(Edit),
    Create(Created),
}

impl CodeBlock {
    pub(crate) fn new(change: Change, now: Instant) -> Option<Self> {
        let language = CodeLanguage::of(&change.path);
        let (body, folds) = if change.created {
            let created = Created::of(&change.path, change.hunks, change.neighbours);
            if created.is_empty() {
                return None;
            }
            (Body::Create(created), 1)
        } else {
            let edit = build::edit_of(change.hunks);
            if edit.lines.is_empty() {
                return None;
            }
            let folds = edit.folds;
            (Body::Edit(edit), folds)
        };
        Some(Self {
            path: change.path,
            language,
            started: now,
            body,
            open: vec![false; folds],
        })
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn row_count(&self) -> usize {
        match &self.body {
            Body::Edit(edit) => edit.row_count(&self.open),
            Body::Create(created) => created.row_count(self.is_open()),
        }
    }

    pub(crate) fn lasts_until(&self) -> Instant {
        let duration = match &self.body {
            Body::Edit(edit) => edit.duration(),
            Body::Create(created) => created.duration(),
        };
        self.started + duration
    }

    pub(crate) fn toggle(&mut self, row: usize) -> bool {
        let fold = match &self.body {
            Body::Edit(edit) => edit.fold_at(row, &self.open),
            Body::Create(created) => created.folds_at(row, self.is_open()).then_some(0),
        };
        let Some(open) = fold.and_then(|fold| self.open.get_mut(fold)) else {
            return false;
        };
        *open = !*open;
        true
    }

    pub(crate) fn opens_file(&self, row: usize, now: Instant) -> bool {
        match &self.body {
            Body::Edit(_) => row == 0,
            Body::Create(created) => created.opens_file(row, self.elapsed(now)),
        }
    }

    pub(crate) fn draw(&self, row: usize, sight: Sight<'_>, place: &mut dyn FnMut(Drawn)) {
        let look = Look {
            elapsed: self.elapsed(sight.now),
            hovered: sight.hovered,
            room: sight.room,
            words: sight.words,
            name: self.path.rsplit(FOLDER_MARK).next().unwrap_or(&self.path),
            language: self.language,
            open: &self.open,
        };
        match &self.body {
            Body::Edit(edit) => edit.draw(row, &look, place),
            Body::Create(created) => created.draw(row, &look, place),
        }
    }

    fn is_open(&self) -> bool {
        self.open.first().copied().unwrap_or(false)
    }

    fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }
}

/// Returns the inks of a line of code in `language`: names in full ink, keywords at 64 %,
/// punctuation and comments at 40 %, pieces in order and covering the whole line.
#[must_use]
pub fn code_inks(text: &str, language: CodeLanguage) -> Vec<(Range<usize>, Ink)> {
    inked(text, language)
}

fn gutter(mark: &str, number: &str, width: usize, ink: Ink) -> Span {
    Span::new(format!("{mark}{number:>width$}{BAR}"), ink)
}

fn folded(width: usize, count: u64, ink: Ink, words: &dyn CodeWords) -> Line {
    folded_row(width, &format!("{count} {}", words.lines(count)), ink)
}

fn folded_row(width: usize, label: &str, ink: Ink) -> Line {
    Line::new(
        0,
        vec![
            gutter(NO_MARK, &DOT.repeat(width.min(MOST_DOTS)), width, ink),
            Span::new(format!("{FOLDED}{label}"), ink),
        ],
    )
}

fn sliced(text: &str, language: CodeLanguage, range: Range<usize>) -> Vec<Span> {
    inked(text, language)
        .into_iter()
        .filter_map(|(piece, ink)| {
            let start = piece.start.max(range.start);
            let end = piece.end.min(range.end);
            (start < end).then(|| Span::new(&text[start..end], ink))
        })
        .collect()
}

fn pace(every: Duration, within: Duration, count: usize) -> Duration {
    every.min(within / count32(count).max(1))
}

fn count32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/code/mod.rs"]
pub(crate) mod tests;
