use std::time::Duration;

use crate::code::build::{KEPT_LINES, digits};
use crate::code::{
    ChangeHunk, ChangeLine, Drawn, Look, NO_MARK, Neighbours, count32, folded, folded_row, gutter,
    pace, sliced,
};
use crate::ink::{Ink, OPAQUE};
use crate::line::{Line, Span, TEXT_INDENT, truncated};
use crate::motion::SETTLE;
use crate::transcript::heat_after;

const VISIBLE: usize = 10;
const TREE: Duration = Duration::from_millis(300);
const LINE_EVERY: Duration = Duration::from_millis(80);
const WRITING: Duration = Duration::from_millis(2_000);
const CREATED: &str = "✚ ";
const WHILE_WRITING: &str = "   ";
const OPENS: &str = " ↗ ";
const SEPARATOR: &str = " · ";
const SIBLING_INDENT: u16 = 4;
const FOLDER_MARK: char = '/';
const HERE: &str = "./";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Created {
    folder: String,
    before: Vec<String>,
    after: Vec<String>,
    lines: Vec<String>,
    beyond: u64,
}

impl Created {
    pub(super) fn of(path: &str, hunks: Vec<ChangeHunk>, neighbours: Option<Neighbours>) -> Self {
        let mut lines: Vec<String> = hunks
            .into_iter()
            .flat_map(|hunk| hunk.lines)
            .filter_map(|line| match line {
                ChangeLine::Added(text) | ChangeLine::Same(text) => Some(text),
                ChangeLine::Removed(_) => None,
            })
            .collect();
        let beyond = u64::try_from(lines.len().saturating_sub(KEPT_LINES)).unwrap_or(u64::MAX);
        lines.truncate(KEPT_LINES);
        lines.shrink_to_fit();
        let Neighbours {
            folder,
            before,
            after,
        } = neighbours.unwrap_or_else(|| Neighbours {
            folder: folder_of(path),
            before: Vec::new(),
            after: Vec::new(),
        });
        Self {
            folder,
            before,
            after,
            lines,
            beyond,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub(super) fn row_count(&self, open: bool) -> usize {
        let shown = if open {
            self.lines.len()
        } else {
            self.lines.len().min(VISIBLE)
        };
        self.head_rows() + shown + self.tail_rows(open)
    }

    pub(super) fn duration(&self) -> Duration {
        TREE + self.every().saturating_mul(count32_of(self.total())) + SETTLE.duration() * 2
    }

    pub(super) fn folds_at(&self, row: usize, open: bool) -> bool {
        let Some(tail) = row.checked_sub(self.head_rows() + self.shown(open)) else {
            return false;
        };
        match (self.folds(), open) {
            (false, _) => false,
            (true, false) => tail == 0,
            (true, true) => tail + 1 == self.tail_rows(true),
        }
    }

    pub(super) fn opens_file(&self, row: usize, elapsed: Duration) -> bool {
        row == self.name_row() && self.written(elapsed) == self.total()
    }

    pub(super) fn draw(&self, row: usize, look: &Look<'_>, emit: &mut dyn FnMut(Drawn)) {
        let open = look.open.first().copied().unwrap_or(false);
        let written = self.written(look.elapsed);
        let writing = written < self.total();
        if row == 0 {
            emit(Drawn::Still(plain(
                TEXT_INDENT,
                &self.folder,
                Ink::Secondary,
            )));
        } else if row < self.name_row() {
            emit(Drawn::Still(plain(
                SIBLING_INDENT,
                &self.before[row - 1],
                Ink::Receding,
            )));
        } else if row == self.name_row() {
            emit(Drawn::Still(self.name_line(look, written)));
        } else if let Some(sibling) = self.after.get(row - self.name_row() - 1) {
            emit(Drawn::Still(plain(SIBLING_INDENT, sibling, Ink::Receding)));
        } else if let Some(line) = row
            .checked_sub(self.head_rows())
            .filter(|&line| line < self.shown(open))
        {
            self.draw_line(line, look, emit);
        } else if let Some(tail) = row.checked_sub(self.head_rows() + self.shown(open)) {
            let width = digits(self.total());
            let hidden = written.saturating_sub(count64(self.shown_of(written, open)));
            let line = match (open, tail) {
                (false, _) if hidden > 0 && writing => {
                    heated(folded(width, hidden, Ink::Full, look.words), OPAQUE)
                }
                (false, _) if hidden > 0 => folded(width, hidden, Ink::Receding, look.words),
                (true, 0) if self.beyond > 0 => folded(width, self.beyond, Ink::Faint, look.words),
                (true, _) => folded_row(width, &look.words.fold_back(), Ink::Receding),
                (false, _) => return,
            };
            emit(Drawn::Still(line));
        }
    }

    fn draw_line(&self, line: usize, look: &Look<'_>, emit: &mut dyn FnMut(Drawn)) {
        let start = TREE + self.every().saturating_mul(count32_of(count64(line)));
        let Some(since) = look.elapsed.checked_sub(start) else {
            return;
        };
        let width = digits(self.total());
        let number = (line + 1).to_string();
        let gutter_span = gutter(NO_MARK, &number, width, Ink::Receding);
        let indent = u16::try_from(gutter_span.columns()).unwrap_or(u16::MAX);
        let room = look.room.saturating_sub(usize::from(indent));
        let text = truncated(&self.lines[line], room);
        let code = sliced(&text, look.language, 0..text.len());
        if let Some(eased) = SETTLE.eased(since) {
            emit(Drawn::Still(Line::new(0, vec![gutter_span])));
            let heat = heat_after(since, SETTLE.duration());
            let spans = code.into_iter().map(|span| span.heated(heat)).collect();
            emit(Drawn::Rising(Line::new(indent, spans), eased));
        } else {
            let mut spans = vec![gutter_span];
            spans.extend(code);
            emit(Drawn::Still(Line::new(0, spans)));
        }
    }

    fn name_line(&self, look: &Look<'_>, written: u64) -> Line {
        let writing = written < self.total();
        let count = format!(
            "{}{SEPARATOR}{written} {}",
            look.words.new_file(),
            look.words.lines(written)
        );
        let count = if writing {
            Span::new(count, Ink::Full).heated(OPAQUE)
        } else {
            Span::new(count, Ink::Secondary)
        };
        let after = if writing { WHILE_WRITING } else { OPENS };
        Line::new(
            TEXT_INDENT,
            vec![
                Span::new(format!("{CREATED}{}", look.name), Ink::Full),
                Span::new(after, Ink::Secondary),
                count,
            ],
        )
    }

    fn written(&self, elapsed: Duration) -> u64 {
        let Some(writing) = elapsed.checked_sub(TREE) else {
            return 0;
        };
        let every = self.every().as_nanos().max(1);
        let lines = writing.as_nanos() / every + 1;
        u64::try_from(lines).unwrap_or(u64::MAX).min(self.total())
    }

    fn shown_of(&self, written: u64, open: bool) -> usize {
        let written = usize::try_from(written)
            .unwrap_or(usize::MAX)
            .min(self.lines.len());
        if open { written } else { written.min(VISIBLE) }
    }

    fn shown(&self, open: bool) -> usize {
        self.shown_of(self.total(), open)
    }

    fn tail_rows(&self, open: bool) -> usize {
        match (self.folds(), open) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 1 + usize::from(self.beyond > 0),
        }
    }

    fn folds(&self) -> bool {
        self.total() > count64(VISIBLE)
    }

    fn every(&self) -> Duration {
        pace(
            LINE_EVERY,
            WRITING,
            usize::try_from(self.total()).unwrap_or(usize::MAX),
        )
    }

    fn total(&self) -> u64 {
        count64(self.lines.len()) + self.beyond
    }

    fn name_row(&self) -> usize {
        1 + self.before.len()
    }

    fn head_rows(&self) -> usize {
        self.name_row() + self.after.len() + 2
    }
}

fn heated(mut line: Line, heat: u8) -> Line {
    for span in &mut line.spans {
        span.heat = heat;
    }
    line
}

fn plain(indent: u16, text: &str, ink: Ink) -> Line {
    Line::new(indent, vec![Span::new(text, ink)])
}

fn folder_of(path: &str) -> String {
    path.rsplit_once(FOLDER_MARK)
        .and_then(|(folder, _)| folder.rsplit(FOLDER_MARK).next())
        .filter(|folder| !folder.is_empty())
        .map_or_else(
            || HERE.to_owned(),
            |folder| format!("{folder}{FOLDER_MARK}"),
        )
}

fn count64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

fn count32_of(count: u64) -> u32 {
    count32(usize::try_from(count).unwrap_or(usize::MAX))
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/code/create.rs"]
mod tests;
