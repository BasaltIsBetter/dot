use std::ops::Range;
use std::time::{Duration, Instant};

use dot_api::{Permission, Text, Translator};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::canvas::{Ink, Line, Placed, Span};
use crate::conversation::flow::tool::revealed;
use crate::conversation::scroll::{Scroll, rows};
use crate::conversation::wrap::wrap;
use crate::conversation::zone::{Band, TextWidth};
use crate::conversation::{MARKER, TEXT_INDENT};
use crate::reply::Answer;
use crate::status_bar::truncated;

const ANSWER_GUARD: Duration = Duration::from_millis(300);
const KEY_GAP: &str = "   ";
const MARKER_INK: Ink = Ink::Live;
const QUESTION_INK: Ink = Ink::Secondary;
const SUBJECT_INK: Ink = Ink::Full;
const KEYS_INK: Ink = Ink::Receding;
const ROWS_AROUND_THE_SUBJECT: usize = 2;

#[derive(Debug)]
pub(crate) struct Question {
    wording: String,
    subject: String,
    keys: String,
    subject_rows: Vec<Range<usize>>,
    width: TextWidth,
    scroll: Scroll,
    asked_at: Instant,
}

impl Question {
    pub(crate) fn new(
        permission: &Permission,
        width: TextWidth,
        now: Instant,
        translator: &impl Translator,
    ) -> Self {
        let action = &permission.action;
        let keys = [
            Text::PermissionAllow,
            Text::PermissionDeny,
            action.grant().text(),
        ]
        .map(|key| translator.translate(key).into_owned())
        .join(KEY_GAP);
        let subject = revealed(action.subject());
        let mut question = Self {
            wording: translator.translate(action.text()).into_owned(),
            subject_rows: wrap(&subject, width.columns()),
            subject,
            keys,
            width,
            scroll: Scroll::default(),
            asked_at: now,
        };
        question.scroll.scroll_to(0.0, question.rows(), 0.0);
        question
    }

    pub(crate) fn rows(&self) -> usize {
        self.subject_rows.len() + ROWS_AROUND_THE_SUBJECT
    }

    pub(crate) fn rewrap_to(&mut self, width: TextWidth) {
        if self.width == width {
            return;
        }
        self.width = width;
        self.subject_rows = wrap(&self.subject, width.columns());
    }

    pub(crate) fn scroll_by(&mut self, rows_up: f64, shown: usize) {
        self.scroll.by(rows_up, self.rows(), rows(shown));
    }

    pub(crate) fn answer_to(
        &self,
        key: &Key,
        modifiers: ModifiersState,
        now: Instant,
    ) -> Option<Answer> {
        if now.saturating_duration_since(self.asked_at) < ANSWER_GUARD {
            return None;
        }
        if modifiers.super_key() || modifiers.control_key() || modifiers.alt_key() {
            return None;
        }
        match key {
            Key::Named(NamedKey::Enter) if !modifiers.shift_key() => Some(Answer::Allow),
            Key::Named(NamedKey::Tab) if !modifiers.shift_key() => Some(Answer::Always),
            Key::Named(NamedKey::Escape) => Some(Answer::Deny),
            _ => None,
        }
    }

    pub(crate) fn lines(&self, band: Band, shown: usize, line_height: f32) -> Vec<Placed> {
        let window = self.scroll.window(self.rows(), rows(shown));
        (window.first..window.end)
            .map(|row| Placed {
                top: band.row_top(window.offset + rows(row - window.first), line_height),
                line: self.row_line(row),
                clip: None,
            })
            .collect()
    }

    fn row_line(&self, row: usize) -> Line {
        let columns = self.width.columns();
        if row == 0 {
            let wording = truncated(&self.wording, columns);
            return Line::new(
                0,
                vec![
                    Span::new(MARKER, MARKER_INK),
                    Span::new(wording, QUESTION_INK),
                ],
            );
        }
        match self.subject_rows.get(row - 1) {
            Some(range) => Line::new(
                TEXT_INDENT,
                vec![Span::new(&self.subject[range.clone()], SUBJECT_INK)],
            ),
            None => Line::new(
                TEXT_INDENT,
                vec![Span::new(truncated(&self.keys, columns), KEYS_INK)],
            ),
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/question.rs"]
mod tests;
