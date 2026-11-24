use crate::ink::{Ink, OPAQUE, Palette, dim, opacity};
use crate::reading::to_f32;

/// Columns between the left edge of the column and the text of every line.
pub const TEXT_INDENT: u16 = 2;
/// Mark in front of what the user sent, in the columns that [`TEXT_INDENT`] leaves.
pub const MARKER: &str = "› ";
/// Mark in front of a note the user slipped into a reply, in the same columns as [`MARKER`].
pub const NOTE_MARKER: &str = "‖ ";
/// Width of the conversation's column at its widest, in characters, [`TEXT_INDENT`] included.
pub const WIDEST_COLUMN: u16 = 96;

const ELLIPSIS: char = '…';

/// A piece of text in one ink, warmed toward orange by its heat.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// The text, one column per character.
    pub text: String,
    /// The ink it settles in.
    pub ink: Ink,
    /// How far it is mixed toward orange: 0 is its ink, 255 is orange.
    pub heat: u8,
}

/// A row of spans that follow each other, from `indent` columns.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Line {
    /// Columns before the first span.
    pub indent: u16,
    /// The spans, left to right.
    pub spans: Vec<Span>,
    /// How much of its ink the line shows, from 0 (nothing) to [`OPAQUE`] (all of it).
    pub alpha: u8,
}

/// A line placed in a pane, to the pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// Top of the line, in pixels.
    pub top: f32,
    /// What it shows.
    pub line: Line,
    /// The band it is cut to, top and bottom in pixels, narrower than its pane's; `None` for
    /// the pane's own.
    pub clip: Option<[f32; 2]>,
}

/// A horizontal band of a pane, top and bottom in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    /// Top, in pixels.
    pub top: f32,
    /// Bottom, in pixels.
    pub bottom: f32,
}

/// How many columns the text of a line can take: the column less [`TEXT_INDENT`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextWidth(usize);

impl Span {
    /// Creates a span of `text` in `ink`, cold.
    pub fn new(text: impl Into<String>, ink: Ink) -> Self {
        Self {
            text: text.into(),
            ink,
            heat: 0,
        }
    }

    /// Returns the same span at `heat`.
    #[must_use]
    pub fn heated(self, heat: u8) -> Self {
        Self { heat, ..self }
    }

    /// Returns how many columns the span takes.
    #[must_use]
    pub fn columns(&self) -> usize {
        columns(&self.text)
    }

    /// Returns the sRGB color of the span in `palette`: its ink mixed toward the live colour by
    /// its heat.
    #[must_use]
    pub fn rgb(&self, palette: &Palette) -> [u8; 3] {
        palette.shade(self.ink, self.heat, OPAQUE)
    }
}

impl Line {
    /// Creates a line of `spans` from `indent` columns, drawn with all of its ink.
    #[must_use]
    pub const fn new(indent: u16, spans: Vec<Span>) -> Self {
        Self {
            indent,
            spans,
            alpha: OPAQUE,
        }
    }

    /// Returns the same line showing only `share` of what it showed, from 0 to 1.
    #[must_use]
    pub fn faded(self, share: f64) -> Self {
        let alpha = dim(self.alpha, opacity(share));
        Self { alpha, ..self }
    }

    /// Returns whether the line shows any text: a span with text, and some of its ink.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.alpha > 0 && self.spans.iter().any(|span| !span.text.is_empty())
    }

    /// Returns the sRGB color `span` is drawn in on this line, in `palette`: its color,
    /// darkened toward the black background as much as the line is faded.
    #[must_use]
    pub fn rgb_of(&self, span: &Span, palette: &Palette) -> [u8; 3] {
        palette.shade(span.ink, span.heat, self.alpha)
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new(0, Vec::new())
    }
}

impl Band {
    /// Returns how many rows of `line_height` pixels fit in the band, zero when none does.
    #[must_use]
    pub fn rows(self, line_height: f32) -> f64 {
        if line_height <= 0.0 {
            return 0.0;
        }
        f64::from(((self.bottom - self.top) / line_height).max(0.0))
    }

    /// Returns the top of the row `rows_down` rows below the top of the band, to the pixel.
    #[must_use]
    pub fn row_top(self, rows_down: f64, line_height: f32) -> f32 {
        (self.top + to_f32(rows_down) * line_height).round()
    }
}

impl TextWidth {
    /// Creates the width of a text that takes `columns` columns.
    #[must_use]
    pub const fn new(columns: usize) -> Self {
        Self(columns)
    }

    /// Returns the number of columns.
    #[must_use]
    pub const fn columns(self) -> usize {
        self.0
    }
}

impl Default for TextWidth {
    fn default() -> Self {
        Self(usize::from(WIDEST_COLUMN - TEXT_INDENT))
    }
}

/// Returns how many columns `text` takes on the grid: one per character.
#[must_use]
pub fn columns(text: &str) -> usize {
    text.chars().count()
}

/// Returns `text` cut to `longest` columns, its last kept column turned into `…` when it is cut.
#[must_use]
pub fn truncated(text: &str, longest: usize) -> String {
    let mut shown = String::new();
    push_truncated(&mut shown, text, longest);
    shown
}

pub(crate) fn push_truncated(shown: &mut String, text: &str, longest: usize) {
    if columns(text) <= longest {
        shown.push_str(text);
        return;
    }
    if longest == 0 {
        return;
    }
    shown.extend(text.chars().take(longest - 1));
    shown.push(ELLIPSIS);
}

#[cfg(test)]
#[path = "../../../tests/dot-transcript/line.rs"]
mod tests;
