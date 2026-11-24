//! Text the reader selects in the conversation, anchored to its rows so that it stays on the
//! same words while the conversation scrolls or grows.

mod rows;

use crate::transcript::Transcript;

pub use self::rows::RowText;

const NEW_LINE: char = '\n';
const SPACE: char = ' ';

/// A place between two characters of the conversation: a row of the history and the count of
/// characters before it on that row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spot {
    /// The row, counted from the first row of the history.
    pub row: usize,
    /// The characters of the row's text before the place.
    pub column: usize,
}

/// What the reader selected: from where the drag started to where it is now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Where the selection started.
    pub anchor: Spot,
    /// Where it ends now, before or after the anchor.
    pub head: Spot,
}

impl Selection {
    /// Returns a selection that starts and ends at `spot`.
    #[must_use]
    pub const fn at(spot: Spot) -> Self {
        Self {
            anchor: spot,
            head: spot,
        }
    }

    /// Returns the first and the last place of the selection, in reading order.
    #[must_use]
    pub fn ends(&self) -> (Spot, Spot) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Returns whether the selection holds no character.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns the columns of `row` the selection covers, `None` when it leaves the row out;
    /// the end of a row is `usize::MAX` when the selection goes on past it.
    #[must_use]
    pub fn columns_on(&self, row: usize) -> Option<(usize, usize)> {
        let (start, end) = self.ends();
        if row < start.row || row > end.row || self.is_empty() {
            return None;
        }
        let from = if row == start.row { start.column } else { 0 };
        let to = if row == end.row {
            end.column
        } else {
            usize::MAX
        };
        Some((from, to))
    }

    /// Returns the same selection grown to whole words at both ends.
    #[must_use]
    pub fn to_words(&self, transcript: &Transcript) -> Self {
        let (start, end) = self.ends();
        let start = word_start(transcript, start);
        let end = word_end(transcript, end);
        if self.anchor <= self.head {
            Self {
                anchor: start,
                head: end,
            }
        } else {
            Self {
                anchor: end,
                head: start,
            }
        }
    }

    /// Returns the text selected as it reads: the rows of a paragraph joined by a space, a new
    /// line between paragraphs, no marker of a question nor number of a line of code.
    #[must_use]
    pub fn text(&self, transcript: &Transcript) -> String {
        let (start, end) = self.ends();
        let mut text = String::new();
        for row in start.row..=end.row {
            let Some((from, to)) = self.columns_on(row) else {
                continue;
            };
            let Some(shown) = transcript.row_text(row) else {
                if row != end.row {
                    text.push(NEW_LINE);
                }
                continue;
            };
            let piece: String = shown
                .text
                .chars()
                .skip(from)
                .take(to.saturating_sub(from))
                .collect();
            if row == end.row {
                text.push_str(&piece);
            } else if shown.flows_on {
                text.push_str(piece.trim_end());
                text.push(SPACE);
            } else {
                text.push_str(piece.trim_end());
                text.push(NEW_LINE);
            }
        }
        text
    }

    /// Returns how many characters and how many rows the selection holds.
    #[must_use]
    pub fn size(&self, transcript: &Transcript) -> (usize, usize) {
        let (start, end) = self.ends();
        let characters = self.text(transcript).chars().count();
        (characters, end.row - start.row + 1)
    }
}

/// Returns the word around `spot`, or the single character there when it is not part of a word.
#[must_use]
pub fn word_at(transcript: &Transcript, spot: Spot) -> Selection {
    let Some(shown) = transcript.row_text(spot.row) else {
        return Selection::at(spot);
    };
    let characters: Vec<char> = shown.text.chars().collect();
    let Some(&character) = characters.get(spot.column) else {
        return Selection::at(spot);
    };
    if !is_word(character) {
        return Selection {
            anchor: spot,
            head: Spot {
                column: spot.column + 1,
                ..spot
            },
        };
    }
    let end = characters[spot.column..]
        .iter()
        .position(|&character| !is_word(character))
        .map_or(characters.len(), |after| spot.column + after);
    Selection {
        anchor: word_start(transcript, spot),
        head: Spot {
            column: end,
            ..spot
        },
    }
}

/// Returns the paragraph `row` belongs to: every row of it, from the first character to the
/// last.
#[must_use]
pub fn paragraph_at(transcript: &Transcript, row: usize) -> Selection {
    let mut first = row;
    while let Some(previous) = first.checked_sub(1)
        && transcript
            .row_text(previous)
            .is_some_and(|shown| shown.flows_on)
    {
        first = previous;
    }
    let mut last = row;
    while transcript
        .row_text(last)
        .is_some_and(|shown| shown.flows_on)
    {
        last += 1;
    }
    let end = transcript
        .row_text(last)
        .map_or(0, |shown| shown.text.trim_end().chars().count());
    Selection {
        anchor: Spot {
            row: first,
            column: 0,
        },
        head: Spot {
            row: last,
            column: end,
        },
    }
}

fn word_start(transcript: &Transcript, spot: Spot) -> Spot {
    let Some(shown) = transcript.row_text(spot.row) else {
        return spot;
    };
    let characters: Vec<char> = shown.text.chars().collect();
    let mut column = spot.column.min(characters.len());
    while column > 0
        && is_word(characters[column - 1])
        && characters
            .get(column)
            .is_some_and(|&character| is_word(character))
    {
        column -= 1;
    }
    Spot { column, ..spot }
}

fn word_end(transcript: &Transcript, spot: Spot) -> Spot {
    let Some(shown) = transcript.row_text(spot.row) else {
        return spot;
    };
    let characters: Vec<char> = shown.text.chars().collect();
    let mut column = spot.column.min(characters.len());
    while column > 0
        && column < characters.len()
        && is_word(characters[column - 1])
        && is_word(characters[column])
    {
        column += 1;
    }
    Spot { column, ..spot }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/selection/mod.rs"]
mod tests;
