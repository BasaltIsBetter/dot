use std::mem;
use std::time::Instant;

use dot_api::{ImageError, Prompt};

use crate::conversation::blink::Caret;
use crate::conversation::prompter::attachment::Attachment;
use crate::conversation::prompter::draft::{Draft, Edit, Motion};
use crate::conversation::prompter::mark::Mark;
use crate::conversation::suggestion::Hint;
use dot_transcript::shown;
use dot_transcript::{Ink, Line, Placed, Span, TEXT_INDENT, TextWidth, columns, truncated};

const CARET: &str = "_";
const TEXT_INK: Ink = Ink::Full;
const ATTACHMENT_INK: Ink = Ink::Secondary;

#[derive(Debug, Default)]
pub(crate) struct Prompter {
    draft: Draft,
    composing: String,
    attachments: Vec<Attachment>,
}

impl Prompter {
    pub(crate) fn rows(&self) -> usize {
        self.draft.rows().len() + self.attachments.len()
    }

    pub(crate) const fn width(&self) -> TextWidth {
        self.draft.width()
    }

    pub(crate) fn rewrap_to(&mut self, width: TextWidth) {
        self.draft.rewrap_to(width);
    }

    pub(crate) fn edit(&mut self, edit: Edit, now: Instant) {
        let erases_before_the_text = edit == Edit::Erase(Motion::Left)
            && self.draft.caret_cell() == (0, 0)
            && !self.attachments.is_empty();
        if erases_before_the_text {
            self.attachments.pop();
            return;
        }
        self.draft.apply(edit, now);
    }

    pub(crate) fn attach(&mut self, attachment: Attachment) -> Result<(), ImageError> {
        let attached = self.attachments.iter().map(|attached| &attached.image);
        Prompt::room_for(attached, &attachment.image)?;
        self.attachments.push(attachment);
        Ok(())
    }

    pub(crate) fn compose(&mut self, text: String) {
        self.composing = text;
    }

    pub(crate) fn take(&mut self) -> Option<(String, Vec<Attachment>)> {
        let text = self.draft.take();
        if text.is_none() && self.attachments.is_empty() {
            return None;
        }
        self.draft.clear();
        Some((text.unwrap_or_default(), mem::take(&mut self.attachments)))
    }

    pub(crate) fn place(&mut self, row: usize, column: usize) {
        if let Some(attachment) = row.checked_sub(self.draft.rows().len()) {
            if attachment < self.attachments.len() {
                self.attachments.remove(attachment);
            }
            return;
        }
        self.draft
            .place(row, column.saturating_sub(usize::from(TEXT_INDENT)));
    }

    pub(crate) fn caret_cell(&self) -> (usize, usize) {
        self.draft.caret_cell()
    }

    pub(crate) fn is_empty(&self) -> bool {
        let empty = self.draft.text().is_empty() && self.composing.is_empty();
        empty && self.attachments.is_empty()
    }

    pub(crate) fn is_cooling(&self, now: Instant) -> bool {
        self.draft.typed().is_cooling(now)
    }

    pub(crate) fn lines(
        &self,
        (top_of, row_height): (impl Fn(usize) -> f32, f32),
        (caret, mark): (Caret, Mark),
        hint: Option<&Hint>,
        now: Instant,
    ) -> Vec<Placed> {
        if self.is_empty() {
            return self.empty_lines((&top_of, row_height), (caret, mark), hint);
        }
        let draft_rows = self.draft.rows().len();
        let mut placed: Vec<Placed> = (0..draft_rows)
            .map(|row| still(top_of(row), self.row_line(row, mark, now)))
            .collect();
        placed.extend(
            self.attachments
                .iter()
                .enumerate()
                .map(|(index, attachment)| {
                    let label = Span::new(attachment.label(self.width()), ATTACHMENT_INK);
                    still(
                        top_of(draft_rows + index),
                        Line::new(TEXT_INDENT, vec![label]),
                    )
                }),
        );

        let (row, column) = self.draft.caret_cell();
        if let Some(span) = self.caret(caret, mark) {
            placed.push(still(top_of(row), Line::new(indent_at(column), vec![span])));
        }
        placed
    }

    fn empty_lines(
        &self,
        (top_of, row_height): (&impl Fn(usize) -> f32, f32),
        (caret, mark): (Caret, Mark),
        hint: Option<&Hint>,
    ) -> Vec<Placed> {
        let marker = still(
            top_of(0),
            Line::new(0, vec![Span::new(mark.text().trim_end(), mark.ink())]),
        );
        let Some(hint) = hint else {
            return vec![marker];
        };
        let room = self.width().columns().saturating_sub(CARET.len());
        let line_of = |text: &str| {
            Line::new(
                TEXT_INDENT,
                vec![Span::new(truncated(text, room), mark.hint_ink())],
            )
        };
        let mut placed = vec![marker];
        placed.extend(hint.placed(top_of(0), row_height, line_of));
        let after_the_hint = columns(&truncated(&hint.text, room));
        if let Some(span) = self.caret(caret, mark) {
            placed.push(still(
                top_of(0),
                Line::new(indent_at(after_the_hint), vec![span]),
            ));
        }
        placed
    }

    fn row_line(&self, row: usize, mark: Mark, now: Instant) -> Line {
        let range = self.draft.rows()[row].clone();
        let text = self.draft.text();
        let mut spans: Vec<Span> = self
            .draft
            .typed()
            .runs(range, now)
            .into_iter()
            .map(|(piece, heat)| Span::new(shown(&text[piece]), TEXT_INK).heated(heat))
            .collect();
        if row > 0 {
            return Line::new(TEXT_INDENT, spans);
        }
        spans.insert(0, Span::new(mark.text(), mark.ink()));
        Line::new(0, spans)
    }

    fn caret(&self, caret: Caret, mark: Mark) -> Option<Span> {
        if !self.composing.is_empty() {
            return Some(Span::new(self.composing.as_str(), Ink::Live));
        }
        match caret {
            Caret::Lit => Some(Span::new(CARET, mark.caret_ink())),
            Caret::Receded => Some(Span::new(CARET, Ink::Receding)),
            Caret::Dark => None,
        }
    }
}

fn still(top: f32, line: Line) -> Placed {
    Placed {
        top,
        line,
        clip: None,
    }
}

fn indent_at(column: usize) -> u16 {
    TEXT_INDENT.saturating_add(u16::try_from(column).unwrap_or(u16::MAX))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/field.rs"]
mod tests;
