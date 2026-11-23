use std::ops::Range;
use std::time::Instant;

use crate::ink::Ink;
use crate::line::{Line, Placed, Span, TEXT_INDENT, columns};
use crate::motion::SETTLE;
use crate::transcript::entry::{Arrival, Entry, Kind};
use crate::transcript::ghost::Ghost;
use crate::transcript::view::place::{Motion, Place};

const CARET: &str = "_";
const CARET_INK: Ink = Ink::Live;
pub(super) const MARKER_INK: Ink = Ink::Receding;

#[derive(Clone, Copy)]
pub(super) struct TextRow<'a> {
    pub(super) entry: &'a Entry,
    pub(super) row: usize,
    pub(super) top: f32,
    pub(super) rolling: Option<&'a Ghost>,
    pub(super) caret: bool,
    pub(super) marker: Option<&'static str>,
    pub(super) alpha: f64,
}

pub(super) fn push_row(
    placed: &mut Vec<Placed>,
    text_row: TextRow<'_>,
    place: Place,
    now: Instant,
) {
    let TextRow {
        entry,
        row,
        top,
        rolling,
        caret,
        marker,
        alpha,
    } = text_row;
    let ink = ink_of(entry.kind);
    let range = entry.row(row);
    let first = placed.len();
    let rise_of = |arrival: Option<&Arrival>| {
        let arrival =
            arrival.filter(|arrival| rolling.is_some_and(|ghost| ghost.rolls(arrival.at)))?;
        SETTLE.eased(now.saturating_duration_since(arrival.at))
    };
    let mut column = 0;
    let mut follows_a_still_piece = false;

    entry.each_piece_by(
        range,
        |arrival| (entry.heat(arrival, now), rise_of(arrival)),
        |piece, (heat, rise)| {
            let text = &entry.text[piece];
            let span = Span::new(text, ink).heated(heat);
            let indent = indent_at(column);
            column += columns(text);
            if let Some(rise) = rise {
                let line = Line::new(indent, vec![span]).faded(alpha);
                placed.push(place.moving(top, Motion::Rising(rise), line));
                follows_a_still_piece = false;
                return;
            }
            match placed.last_mut().filter(|_| follows_a_still_piece) {
                Some(last) => last.line.spans.push(span),
                None => placed.push(Placed {
                    top,
                    line: Line::new(indent, vec![span]).faded(alpha),
                    clip: None,
                }),
            }
            follows_a_still_piece = true;
        },
    );

    if let Some(marker) = marker {
        let line = &mut placed[first].line;
        let heat = line.spans.first().map_or(0, |span| span.heat);
        line.spans
            .insert(0, Span::new(marker, MARKER_INK).heated(heat));
        line.indent = 0;
    }
    if caret && let Some(last) = placed[first..].last_mut() {
        last.line.spans.push(Span::new(CARET, CARET_INK));
    }
}

pub(super) fn pieces(entry: &Entry, bytes: Range<usize>, now: Instant) -> Vec<Span> {
    let ink = ink_of(entry.kind);
    let mut spans = Vec::new();
    entry.each_piece(bytes, now, |piece, heat| {
        spans.push(Span::new(&entry.text[piece], ink).heated(heat));
    });
    spans
}

pub(super) fn indent_at(column: usize) -> u16 {
    TEXT_INDENT.saturating_add(u16::try_from(column).unwrap_or(u16::MAX))
}

const fn ink_of(kind: Kind) -> Ink {
    match kind {
        Kind::Prompt | Kind::Note | Kind::Answer => Ink::Full,
        Kind::Thinking | Kind::Attachment | Kind::Tool => Ink::Secondary,
        Kind::Notice => Ink::Receding,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/row.rs"]
mod tests;
