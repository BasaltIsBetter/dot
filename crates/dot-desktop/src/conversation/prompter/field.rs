use std::mem;

use dot_api::Image;

use crate::canvas::{Bounds, Ink, Layout, Line, Placed, Span};
use crate::conversation::prompter::draft::{Draft, Edit};
use crate::conversation::prompter::motion::Motion;
use crate::conversation::scroll::{Scroll, Window, index, rows};
use crate::conversation::zone::{Band, TextWidth};
use crate::conversation::{MARKER, TEXT_INDENT};
use crate::status_bar::truncated;

const CARET: &str = "_";
const PLACEHOLDER_INDENT: u16 = 4;
const ATTACHMENT_INK: Ink = Ink::Secondary;
const LABEL_GAP: &str = "  ";

#[derive(Debug, Default)]
pub(crate) struct Prompter {
    draft: Draft,
    composing: String,
    attachments: Vec<Attachment>,
    scroll: Scroll,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Attachment {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) size: String,
    pub(crate) image: Image,
}

impl Attachment {
    pub(crate) fn label(&self, width: TextWidth) -> String {
        let gaps = 2 * LABEL_GAP.chars().count();
        let kept = self.kind.chars().count() + self.size.chars().count() + gaps;
        let name = truncated(&self.name, width.columns().saturating_sub(kept));
        [self.kind.as_str(), &name, &self.size].join(LABEL_GAP)
    }
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

    pub(crate) fn single_row_columns(&self, placeholder: &str) -> Option<usize> {
        if self.rows() > 1 {
            return None;
        }
        if self.shows_placeholder() {
            let hint = self.fitted_placeholder(placeholder).chars().count();
            return Some(usize::from(PLACEHOLDER_INDENT) + hint);
        }
        let caret = self.composing.chars().count().max(CARET.len());
        Some(usize::from(TEXT_INDENT) + self.draft.text().chars().count() + caret)
    }

    pub(crate) fn edit(&mut self, edit: Edit) {
        let erases_before_the_text = edit == Edit::Erase(Motion::Left)
            && self.draft.caret_cell() == (0, 0)
            && !self.attachments.is_empty();
        if erases_before_the_text {
            self.attachments.pop();
            return;
        }
        self.draft.apply(edit);
    }

    pub(crate) fn attach(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
    }

    pub(crate) fn reveal_caret(&mut self, shown: usize) {
        let (row, _) = self.draft.caret_cell();
        self.scroll.reveal(row, self.rows(), rows(shown));
    }

    pub(crate) fn compose(&mut self, text: String) {
        self.composing = text;
    }

    pub(crate) fn scroll_by(&mut self, rows_up: f64, shown: usize) {
        self.scroll.by(rows_up, self.rows(), rows(shown));
    }

    pub(crate) fn take(&mut self) -> Option<(String, Vec<Attachment>)> {
        let text = self.draft.take();
        if text.is_none() && self.attachments.is_empty() {
            return None;
        }
        self.draft.clear();
        self.scroll.follow();
        Some((text.unwrap_or_default(), mem::take(&mut self.attachments)))
    }

    pub(crate) fn place(&mut self, [x, y]: [f32; 2], band: Band, shown: usize, layout: &Layout) {
        let window = self.window(shown);
        let down = f64::from((y - band.top) / layout.line_height()) - window.offset;
        let row = (window.first + index(down.floor())).min(window.end.saturating_sub(1));
        if let Some(attachment) = row.checked_sub(self.draft.rows().len()) {
            self.attachments.remove(attachment);
            return;
        }
        let column = index(f64::from((x - layout.x(TEXT_INDENT)) / layout.advance()).round());
        self.draft.place(row, column);
    }

    pub(crate) fn lines(
        &self,
        band: Band,
        shown: usize,
        layout: &Layout,
        focused: bool,
        placeholder: &str,
    ) -> Vec<Placed> {
        let window = self.window(shown);
        let top_of = |row: usize| {
            band.row_top(
                window.offset + rows(row - window.first),
                layout.line_height(),
            )
        };
        let mut placed: Vec<Placed> = (window.first..window.end)
            .map(|row| Placed {
                top: top_of(row),
                line: self.row_line(row),
                clip: None,
            })
            .collect();

        let (caret_row, _) = self.draft.caret_cell();
        if (window.first..window.end).contains(&caret_row) {
            placed.push(Placed {
                top: top_of(caret_row),
                line: Line::new(self.caret_indent(), vec![self.caret(focused)]),
                clip: None,
            });
        }
        if self.shows_placeholder() {
            placed.push(Placed {
                top: top_of(0),
                line: self.placeholder_line(placeholder),
                clip: None,
            });
        }
        placed
    }

    pub(crate) fn caret_area(&self, band: Band, shown: usize, layout: &Layout) -> Bounds {
        let window = self.window(shown);
        let (row, _) = self.draft.caret_cell();
        let rows_down = rows(row) - rows(window.first) + window.offset;
        Bounds {
            left: layout.x(self.caret_indent()),
            top: band.row_top(rows_down, layout.line_height()),
            width: layout.advance(),
            height: layout.line_height(),
        }
    }

    fn shows_placeholder(&self) -> bool {
        let empty = self.draft.text().is_empty() && self.composing.is_empty();
        empty && self.attachments.is_empty()
    }

    fn fitted_placeholder(&self, placeholder: &str) -> String {
        let columns = self.width().columns() + usize::from(TEXT_INDENT);
        truncated(
            placeholder,
            columns.saturating_sub(usize::from(PLACEHOLDER_INDENT)),
        )
    }

    fn placeholder_line(&self, placeholder: &str) -> Line {
        let hint = Span::new(self.fitted_placeholder(placeholder), Ink::Receding);
        Line::new(PLACEHOLDER_INDENT, vec![hint])
    }

    fn caret_indent(&self) -> u16 {
        let (_, column) = self.draft.caret_cell();
        TEXT_INDENT.saturating_add(u16::try_from(column).unwrap_or(u16::MAX))
    }

    fn window(&self, shown: usize) -> Window {
        self.scroll.window(self.rows(), rows(shown))
    }

    fn row_line(&self, row: usize) -> Line {
        let Some(range) = self.draft.rows().get(row) else {
            let label = self.attachments[row - self.draft.rows().len()].label(self.width());
            return Line::new(TEXT_INDENT, vec![Span::new(label, ATTACHMENT_INK)]);
        };
        let content = &self.draft.text()[range.clone()];
        if row == 0 {
            return Line::new(0, vec![Span::new(format!("{MARKER}{content}"), Ink::Full)]);
        }
        Line::new(TEXT_INDENT, vec![Span::new(content, Ink::Full)])
    }

    fn caret(&self, focused: bool) -> Span {
        if !self.composing.is_empty() {
            return Span::new(self.composing.as_str(), Ink::Live);
        }
        Span::new(CARET, if focused { Ink::Full } else { Ink::Receding })
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/field.rs"]
mod tests;
