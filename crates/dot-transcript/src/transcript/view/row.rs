use std::ops::Range;
use std::time::Instant;

use crate::code::middles;
use crate::ink::Ink;
use crate::line::{Line, Placed, Span, TEXT_INDENT, columns};
use crate::motion::SETTLE;
use crate::transcript::entry::{Arrival, Entry, Kind};
use crate::transcript::fade::{heat, is_fresh};
use crate::transcript::ghost::Ghost;
use crate::transcript::tool::Replaced;
use crate::transcript::view::place::{Motion, Place};

const CARET: &str = "_";
const CARET_INK: Ink = Ink::Live;
pub(super) const MARKER_INK: Ink = Ink::Receding;
const NOTE_INK: Ink = Ink::Secondary;

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
    if let Some(replaced) = fresh_replacement(text_row.entry, now) {
        push_replaced(placed, text_row, replaced, place, now);
        return;
    }
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
    let range = entry.shown_row(row);
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
            .insert(0, Span::new(marker, marker_ink(entry.kind)).heated(heat));
        line.indent = 0;
    }
    if caret && let Some(last) = placed[first..].last_mut() {
        last.line.spans.push(Span::new(CARET, CARET_INK));
    }
}

fn fresh_replacement(entry: &Entry, now: Instant) -> Option<&Replaced> {
    entry
        .replaced()
        .filter(|replaced| is_fresh(now.saturating_duration_since(replaced.at)))
        .filter(|_| entry.row_count() == 1)
}

fn push_replaced(
    placed: &mut Vec<Placed>,
    text_row: TextRow<'_>,
    replaced: &Replaced,
    place: Place,
    now: Instant,
) {
    let TextRow {
        entry,
        top,
        caret,
        alpha,
        ..
    } = text_row;
    let ink = ink_of(entry.kind);
    let text = &entry.text[entry.shown_row(0)];
    let (old_middle, new_middle) = middles(&replaced.before, text);
    let since = now.saturating_duration_since(replaced.at);
    let settled = entry.heat(None, now);
    let head = Span::new(&text[..new_middle.start], ink).heated(settled);
    let tail = Span::new(&text[new_middle.end..], ink).heated(settled);
    let arrived = Span::new(&text[new_middle], ink).heated(heat(since));
    let still = |middle: Span| {
        let mut spans: Vec<Span> = [head.clone(), middle, tail.clone()]
            .into_iter()
            .filter(|span| !span.text.is_empty())
            .collect();
        if caret {
            spans.push(Span::new(CARET, CARET_INK));
        }
        Placed {
            top,
            line: Line::new(TEXT_INDENT, spans).faded(alpha),
            clip: None,
        }
    };

    let Some(eased) = SETTLE.eased(since) else {
        placed.push(still(arrived));
        return;
    };
    placed.push(still(Span::new(" ".repeat(columns(&arrived.text)), ink)));
    let indent = indent_at(columns(&head.text));
    let rising = Line::new(indent, vec![arrived]).faded(alpha);
    placed.push(place.moving(top, Motion::Rising(eased), rising));
    let gone = Span::new(&replaced.before[old_middle], ink).heated(settled);
    let leaving = Line::new(indent, vec![gone]).faded(alpha);
    placed.push(place.moving(top, Motion::Leaving(eased), leaving));
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

const fn marker_ink(kind: Kind) -> Ink {
    match kind {
        Kind::Note => NOTE_INK,
        _ => MARKER_INK,
    }
}

const fn ink_of(kind: Kind) -> Ink {
    match kind {
        Kind::Prompt | Kind::Answer => Ink::Full,
        Kind::Note | Kind::Thinking | Kind::Attachment | Kind::Tool => Ink::Secondary,
        Kind::Notice => Ink::Receding,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/row.rs"]
mod tests;
