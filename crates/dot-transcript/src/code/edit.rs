use std::ops::Range;
use std::time::Duration;

use crate::code::ink::tokens;
use crate::code::{
    BAR, CONTEXT_SHARE, Drawn, Look, MARK_WIDTH, NO_MARK, count32, folded, gutter, pace, sliced,
};
use crate::ink::Ink;
use crate::line::{Line, Span, TEXT_INDENT, columns, truncated};
use crate::motion::SETTLE;
use crate::transcript::heat_after;

const LINE_EVERY: Duration = Duration::from_millis(70);
const APPEARING: Duration = Duration::from_millis(1_000);
const CHANGE_EVERY: Duration = Duration::from_millis(450);
const CHANGING: Duration = Duration::from_millis(1_800);
const CHANGED_MARK: &str = "── ";
const ADDED_MARK: &str = "─  ";
const OPENS: &str = " ↗";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Shown {
    Same {
        number: u64,
        text: String,
    },
    Changed {
        number: u64,
        text: String,
        old: String,
        order: usize,
    },
    Added {
        number: u64,
        text: String,
        order: usize,
    },
    Removed {
        old: String,
        order: usize,
    },
    Fold {
        fold: usize,
        lines: Vec<(u64, String)>,
    },
    Gap {
        count: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edit {
    pub(super) lines: Vec<Shown>,
    pub(super) changes: usize,
    pub(super) folds: usize,
    pub(super) width: usize,
}

impl Shown {
    pub(super) fn last_number(&self) -> u64 {
        match self {
            Self::Same { number, .. }
            | Self::Changed { number, .. }
            | Self::Added { number, .. } => *number,
            Self::Fold { lines, .. } => lines.last().map_or(0, |(number, _)| *number),
            Self::Removed { .. } | Self::Gap { .. } => 0,
        }
    }

    fn rows(&self, open: &[bool]) -> usize {
        match self {
            Self::Fold { fold, lines } if is_open(open, *fold) => lines.len(),
            _ => 1,
        }
    }
}

impl Edit {
    pub(super) fn row_count(&self, open: &[bool]) -> usize {
        1 + self.lines.iter().map(|line| line.rows(open)).sum::<usize>()
    }

    pub(super) fn duration(&self) -> Duration {
        let (every, stagger) = self.paces();
        every.saturating_mul(count32(self.lines.len()))
            + stagger.saturating_mul(count32(self.changes.saturating_sub(1)))
            + SETTLE.duration() * 2
    }

    pub(super) fn fold_at(&self, row: usize, open: &[bool]) -> Option<usize> {
        match self.at(row, open)? {
            (Shown::Fold { fold, .. }, ..) => Some(*fold),
            _ => None,
        }
    }

    pub(super) fn draw(&self, row: usize, look: &Look<'_>, emit: &mut dyn FnMut(Drawn)) {
        if row == 0 {
            let spans = vec![
                Span::new(look.name, Ink::Secondary),
                Span::new(OPENS, Ink::Secondary),
            ];
            emit(Drawn::Still(Line::new(TEXT_INDENT, spans)));
            return;
        }
        let Some((shown, index, within)) = self.at(row, look.open) else {
            return;
        };
        let (every, _) = self.paces();
        if look.elapsed < every.saturating_mul(count32(index)) {
            return;
        }
        let settled = look.elapsed >= self.duration();
        let row = Row {
            look,
            width: self.width,
            settled,
        };
        match shown {
            Shown::Same { number, text } => {
                emit(Drawn::Still(row.context(*number, text)));
            }
            Shown::Fold { fold, lines } if is_open(look.open, *fold) => {
                let (number, text) = &lines[within];
                let line = row.line(NO_MARK, &number.to_string(), Ink::Receding, Some(text));
                emit(Drawn::Still(line.faded(CONTEXT_SHARE)));
            }
            Shown::Fold { lines, .. } => {
                let count = u64::try_from(lines.len()).unwrap_or(u64::MAX);
                emit(Drawn::Still(folded(
                    self.width,
                    count,
                    Ink::Receding,
                    look.words,
                )));
            }
            Shown::Gap { count } => {
                emit(Drawn::Still(folded(
                    self.width,
                    *count,
                    Ink::Faint,
                    look.words,
                )));
            }
            Shown::Changed {
                number,
                text,
                old,
                order,
            } => row.changed(*number, (old, text), self.span(*order), emit),
            Shown::Added {
                number,
                text,
                order,
            } => row.added(*number, text, self.span(*order), emit),
            Shown::Removed { old, order } => row.removed(old, self.span(*order), emit),
        }
    }

    fn paces(&self) -> (Duration, Duration) {
        (
            pace(LINE_EVERY, APPEARING, self.lines.len()),
            pace(CHANGE_EVERY, CHANGING, self.changes),
        )
    }

    fn span(&self, order: usize) -> Range<Duration> {
        let (every, stagger) = self.paces();
        let start = every.saturating_mul(count32(self.lines.len()))
            + stagger.saturating_mul(count32(order));
        start..start + SETTLE.duration()
    }

    fn at(&self, row: usize, open: &[bool]) -> Option<(&Shown, usize, usize)> {
        let mut first = 1;
        for (index, shown) in self.lines.iter().enumerate() {
            let rows = shown.rows(open);
            if row < first + rows {
                return Some((shown, index, row.checked_sub(first)?));
            }
            first += rows;
        }
        None
    }
}

struct Row<'a, 'b> {
    look: &'a Look<'b>,
    width: usize,
    settled: bool,
}

impl Row<'_, '_> {
    fn context(&self, number: u64, text: &str) -> Line {
        let line = self.line(NO_MARK, &number.to_string(), Ink::Receding, Some(text));
        if self.settled {
            line.faded(CONTEXT_SHARE)
        } else {
            line
        }
    }

    fn changed(
        &self,
        number: u64,
        (old, new): (&str, &str),
        span: Range<Duration>,
        emit: &mut dyn FnMut(Drawn),
    ) {
        let number = number.to_string();
        let elapsed = self.look.elapsed;
        if self.settled {
            let shown = if self.look.hovered { old } else { new };
            let mut line = self.line(CHANGED_MARK, &number, Ink::Full, None);
            line.spans.extend(self.dimmed_if(self.look.hovered, shown));
            emit(Drawn::Still(line));
            return;
        }
        if elapsed < span.start || elapsed >= span.end {
            let shown = if elapsed < span.start { old } else { new };
            emit(Drawn::Still(self.line(
                NO_MARK,
                &number,
                Ink::Receding,
                Some(shown),
            )));
            return;
        }
        let gutter = self.line(NO_MARK, &number, Ink::Receding, None);
        self.word_by_word(gutter, (old, new), elapsed.saturating_sub(span.start), emit);
    }

    fn added(&self, number: u64, text: &str, span: Range<Duration>, emit: &mut dyn FnMut(Drawn)) {
        let number = number.to_string();
        let elapsed = self.look.elapsed;
        let (mark, ink) = self.mark_of(ADDED_MARK);
        if elapsed >= span.end {
            emit(Drawn::Still(self.line(mark, &number, ink, Some(text))));
            return;
        }
        emit(Drawn::Still(self.line(mark, &number, ink, None)));
        if elapsed >= span.start {
            emit(self.rising(text, 0..text.len(), elapsed.saturating_sub(span.start)));
        }
    }

    fn removed(&self, old: &str, span: Range<Duration>, emit: &mut dyn FnMut(Drawn)) {
        let elapsed = self.look.elapsed;
        let (mark, ink) = self.mark_of(CHANGED_MARK);
        if elapsed < span.start {
            emit(Drawn::Still(self.line(mark, "", ink, Some(old))));
            return;
        }
        let mut line = self.line(mark, "", ink, None);
        if self.settled && self.look.hovered {
            line.spans.extend(self.dimmed_if(true, old));
        }
        emit(Drawn::Still(line));
        if let Some(eased) = SETTLE.eased(elapsed.saturating_sub(span.start)) {
            let old = self.cut(old);
            let spans = sliced(&old, self.look.language, 0..old.len());
            emit(Drawn::Leaving(
                Line::new(self.gutter_columns(), spans),
                eased,
            ));
        }
    }

    fn word_by_word(
        &self,
        gutter: Line,
        (old, new): (&str, &str),
        since: Duration,
        emit: &mut dyn FnMut(Drawn),
    ) {
        let (old, new) = (self.cut(old), self.cut(new));
        let (old_middle, new_middle) = middles(&old, &new);
        let language = self.look.language;
        let mut still = gutter;
        still
            .spans
            .extend(sliced(&new, language, 0..new_middle.start));
        let blank = " ".repeat(columns(&new[new_middle.clone()]));
        still.spans.push(Span::new(blank, Ink::Full));
        still
            .spans
            .extend(sliced(&new, language, new_middle.end..new.len()));
        emit(Drawn::Still(still));

        let eased = SETTLE.eased(since).unwrap_or(1.0);
        emit(self.rising(&new, new_middle.clone(), since));
        let indent = self.gutter_columns() + count16(columns(&new[..new_middle.start]));
        let leaving = Line::new(indent, sliced(&old, language, old_middle));
        emit(Drawn::Leaving(leaving, eased));
    }

    fn rising(&self, text: &str, range: Range<usize>, since: Duration) -> Drawn {
        let text = self.cut(text);
        let range = range.start.min(text.len())..range.end.min(text.len());
        let heat = heat_after(since, SETTLE.duration());
        let indent = self.gutter_columns() + count16(columns(&text[..range.start]));
        let spans = sliced(&text, self.look.language, range)
            .into_iter()
            .map(|span| span.heated(heat))
            .collect();
        Drawn::Rising(Line::new(indent, spans), SETTLE.eased(since).unwrap_or(1.0))
    }

    fn line(&self, mark: &str, number: &str, ink: Ink, text: Option<&str>) -> Line {
        let mut spans = vec![gutter(mark, number, self.width, ink)];
        if let Some(text) = text {
            let text = self.cut(text);
            spans.extend(sliced(&text, self.look.language, 0..text.len()));
        }
        Line::new(0, spans)
    }

    fn dimmed_if(&self, dimmed: bool, text: &str) -> Vec<Span> {
        let text = self.cut(text);
        if dimmed {
            return vec![Span::new(text, Ink::Receding)];
        }
        sliced(&text, self.look.language, 0..text.len())
    }

    fn mark_of(&self, mark: &'static str) -> (&'static str, Ink) {
        if self.settled {
            (mark, Ink::Full)
        } else {
            (NO_MARK, Ink::Receding)
        }
    }

    fn cut(&self, text: &str) -> String {
        let room = self
            .look
            .room
            .saturating_sub(usize::from(self.gutter_columns()));
        truncated(text, room)
    }

    fn gutter_columns(&self) -> u16 {
        count16(MARK_WIDTH + self.width + columns(BAR))
    }
}

fn middles(old: &str, new: &str) -> (Range<usize>, Range<usize>) {
    let old_tokens = tokens(old);
    let new_tokens = tokens(new);
    let same =
        |(left, right): (&Range<usize>, &Range<usize>)| old[left.clone()] == new[right.clone()];
    let head = old_tokens
        .iter()
        .zip(&new_tokens)
        .take_while(|&pair| same(pair))
        .count();
    let tail = old_tokens[head..]
        .iter()
        .rev()
        .zip(new_tokens[head..].iter().rev())
        .take_while(|&pair| same(pair))
        .count();
    let middle = |tokens: &[Range<usize>], text: &str| {
        let start = head.checked_sub(1).map_or(0, |last| tokens[last].end);
        let end = tokens
            .len()
            .checked_sub(tail)
            .filter(|_| tail > 0)
            .map_or(text.len(), |first| tokens[first].start);
        start..end.max(start)
    };
    (middle(&old_tokens, old), middle(&new_tokens, new))
}

fn is_open(open: &[bool], fold: usize) -> bool {
    open.get(fold).copied().unwrap_or(false)
}

fn count16(columns: usize) -> u16 {
    u16::try_from(columns).unwrap_or(u16::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/code/edit.rs"]
mod tests;
