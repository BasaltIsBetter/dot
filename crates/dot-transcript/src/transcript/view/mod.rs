mod ghost;
mod gutter;
mod place;
mod row;

use std::time::Instant;

use crate::code::{CodeBlock, CodeWords, Drawn, Sight};
use crate::line::{Band, MARKER, NOTE_MARKER, Placed, TEXT_INDENT};
use crate::reading::{Window, rows};
use crate::transcript::entry::Kind;
use crate::transcript::history::{Row, Transcript};
use crate::transcript::view::place::{Motion, Place};
use crate::transcript::view::row::{TextRow, push_row};

pub use self::gutter::{Marked, gutter};

const READABLE: f64 = 0.64;
const REVIEW_RADIUS: f64 = 2.0;

/// Where and how a frame shows the conversation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Where rows may be drawn, in pixels.
    pub band: Band,
    /// Height of a row, in pixels.
    pub line_height: f32,
    /// The row at the top of the band, part of a row or negative when the rows start lower.
    pub top: f64,
    /// The row on the reading line while the reader looks back: the rows around it show in
    /// full ink, older ones included.
    pub review: Option<f64>,
    /// Whether the orange `_` that ends the reply being written is lit in this frame.
    pub caret_lit: bool,
    /// The row under the pointer, if any: a line of code changed by Claude shows there what it
    /// was before.
    pub hovered: Option<usize>,
}

/// Returns the lines that show `transcript` at `now` in `view`.
///
/// The latest exchange is in full ink, older ones recede to 64 %, but for the rows around the
/// reading line while the reader looks back. While Claude replies, an orange `_` ends the reply
/// once every word of the question has given way to it, and only when the view says it is lit.
/// The lines of code a tool changed show under it, with `words` around their numbers.
#[must_use]
pub fn lines(
    transcript: &Transcript,
    view: &View,
    now: Instant,
    words: &dyn CodeWords,
) -> Vec<Placed> {
    let window = window(transcript, view);
    let place = place(view, window);
    let active_from = transcript.active_from();
    let returning = transcript.returning(now);
    let shows_caret = view.caret_lit && transcript.shows_caret(now);
    let mut placed = Vec::new();

    for (down, index) in (window.first..window.end).enumerate() {
        let Some(Row::Text {
            entry,
            row,
            position,
        }) = transcript.row(index)
        else {
            continue;
        };
        let returned = returning
            .as_ref()
            .filter(|(question, _)| question.contains(&position))
            .map_or(1.0, |(_, shown)| *shown);
        let alpha = row_alpha(index, active_from, view.review) * returned;
        if let Some(code) = entry.code() {
            let sight = Sight {
                now,
                hovered: view.hovered == Some(index),
                room: usize::from(TEXT_INDENT) + transcript.width().columns(),
                words,
            };
            push_code(
                &mut placed,
                (code, row, sight),
                (place, place.top_at(down)),
                alpha,
            );
            continue;
        }
        let text_row = TextRow {
            entry,
            row,
            top: place.top_at(down),
            rolling: transcript
                .ghost()
                .filter(|_| transcript.is_in_reply(position)),
            caret: shows_caret && transcript.is_last_row(index),
            marker: marker_of(entry.kind).filter(|_| row == 0),
            alpha,
        };
        push_row(&mut placed, text_row, place, now);
    }
    if let (Some(ghost), Some(anchor)) = (transcript.ghost(), transcript.anchor()) {
        ghost::push_ghost(&mut placed, transcript, ghost, (place, anchor), now);
    }
    placed
}

fn push_code(
    placed: &mut Vec<Placed>,
    (code, row, sight): (&CodeBlock, usize, Sight<'_>),
    (place, top): (Place, f32),
    alpha: f64,
) {
    code.draw(row, sight, &mut |drawn| {
        placed.push(match drawn {
            Drawn::Still(line) => Placed {
                top,
                line: line.faded(alpha),
                clip: None,
            },
            Drawn::Rising(line, eased) => {
                place.moving(top, Motion::Rising(eased), line.faded(alpha))
            }
            Drawn::Leaving(line, eased) => {
                place.moving(top, Motion::Leaving(eased), line.faded(alpha))
            }
        });
    });
}

fn window(transcript: &Transcript, view: &View) -> Window {
    Window::at(
        view.top,
        transcript.end_row(),
        view.band.rows(view.line_height),
    )
}

const fn place(view: &View, window: Window) -> Place {
    Place {
        band: view.band,
        line_height: view.line_height,
        first: window.first,
        offset: window.offset,
    }
}

const fn marker_of(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Prompt => Some(MARKER),
        Kind::Note => Some(NOTE_MARKER),
        _ => None,
    }
}

fn row_alpha(index: usize, active_from: usize, review: Option<f64>) -> f64 {
    let looked_at = review.is_some_and(|focus| (rows(index) - focus).abs() <= REVIEW_RADIUS);
    if index >= active_from || looked_at {
        1.0
    } else {
        READABLE
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/mod.rs"]
mod tests;
