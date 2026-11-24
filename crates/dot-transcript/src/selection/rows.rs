use crate::line::TEXT_INDENT;
use crate::transcript::{Row, Transcript};

const NEW_LINE: char = '\n';

/// A row of the conversation as the reader selects and copies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowText<'a> {
    /// The text the row shows, without the marker of a question.
    pub text: &'a str,
    /// The columns before the text, from the left edge of the conversation's column.
    pub indent: u16,
    /// Whether the next row goes on with the same paragraph, cut there only to fit the column.
    pub flows_on: bool,
}

impl Transcript {
    /// Returns the text row `index` of the history shows, `None` for a gap between two entries,
    /// a row of a block of code or a row past the end.
    #[must_use]
    pub fn row_text(&self, index: usize) -> Option<RowText<'_>> {
        let Some(Row::Text { entry, row, .. }) = self.row(index) else {
            return None;
        };
        if entry.code().is_some() {
            return None;
        }
        let range = entry.shown_row(row);
        let text = entry.text[range.clone()].trim_end_matches(NEW_LINE);
        let flows_on = row + 1 < entry.row_count()
            && !entry.text[..entry.row(row + 1).start].ends_with(NEW_LINE);
        Some(RowText {
            text,
            indent: TEXT_INDENT,
            flows_on,
        })
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/selection/rows.rs"]
mod tests;
