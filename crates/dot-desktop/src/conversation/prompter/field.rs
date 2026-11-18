use crate::canvas::{Bounds, Ink, Layout, Line, Placed, Span};
use crate::conversation::prompter::draft::{Draft, Edit};
use crate::conversation::scroll::{Scroll, Window, index, rows};
use crate::conversation::zone::Band;
use crate::conversation::{MARKER, TEXT_INDENT, TEXT_WIDTH};

const CARET: &str = "_";
const PLACEHOLDER_INDENT: u16 = 4;

#[derive(Debug, Default)]
pub(crate) struct Prompter {
    draft: Draft,
    composing: String,
    scroll: Scroll,
}

impl Prompter {
    pub(crate) fn rows(&self) -> usize {
        self.draft.rows().len()
    }

    pub(crate) fn edit(&mut self, edit: Edit) {
        self.draft.apply(edit);
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

    pub(crate) fn take(&mut self) -> Option<String> {
        let sent = self.draft.take()?;
        self.scroll.follow();
        Some(sent)
    }

    pub(crate) fn place(&mut self, [x, y]: [f32; 2], band: Band, shown: usize, layout: &Layout) {
        let window = self.window(shown);
        let down = f64::from((y - band.top) / layout.line_height()) - window.offset;
        let row = (window.first + index(down.floor())).min(window.end.saturating_sub(1));
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
            })
            .collect();

        let (caret_row, _) = self.draft.caret_cell();
        if (window.first..window.end).contains(&caret_row) {
            placed.push(Placed {
                top: top_of(caret_row),
                line: Line::new(self.caret_indent(), vec![self.caret(focused)]),
            });
        }
        if self.draft.text().is_empty() && self.composing.is_empty() {
            placed.push(Placed {
                top: top_of(0),
                line: Line::new(
                    PLACEHOLDER_INDENT,
                    vec![Span::new(placeholder, Ink::Receding)],
                ),
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

    fn caret_indent(&self) -> u16 {
        let (_, column) = self.draft.caret_cell();
        let column = u16::try_from(column.min(TEXT_WIDTH)).unwrap_or(u16::MAX);
        TEXT_INDENT.saturating_add(column)
    }

    fn window(&self, shown: usize) -> Window {
        self.scroll.window(self.rows(), rows(shown))
    }

    fn row_line(&self, row: usize) -> Line {
        let content = &self.draft.text()[self.draft.rows()[row].clone()];
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
