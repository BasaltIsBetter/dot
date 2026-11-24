use std::time::Instant;

use crate::line::MARKER;
use crate::line::{Line, Placed, Span};
use crate::transcript::entry::{Entry, Kind};
use crate::transcript::ghost::{Fate, Ghost, words};
use crate::transcript::history::Transcript;
use crate::transcript::view::place::{Motion, Place};
use crate::transcript::view::row::{MARKER_INK, indent_at, pieces};

pub(super) fn push_ghost(
    placed: &mut Vec<Placed>,
    transcript: &Transcript,
    ghost: &Ghost,
    (place, anchor): (Place, usize),
    now: Instant,
) {
    if ghost.is_settled(now) {
        return;
    }
    let question = ghost
        .question()
        .filter_map(|position| transcript.entry(position));
    let rows = question.flat_map(|entry| (0..entry.row_count()).map(move |row| (entry, row)));

    for (ghost_row, (entry, row)) in rows.enumerate() {
        let Some(top) = place.top_of(anchor + ghost_row) else {
            continue;
        };
        if top >= place.band.bottom {
            return;
        }
        let ghost_line = GhostLine {
            entry,
            row,
            ghost_row,
            top,
        };
        push_ghost_row(placed, ghost_line, ghost, place, now);
    }
}

#[derive(Clone, Copy)]
struct GhostLine<'a> {
    entry: &'a Entry,
    row: usize,
    ghost_row: usize,
    top: f32,
}

fn push_ghost_row(
    placed: &mut Vec<Placed>,
    ghost_line: GhostLine<'_>,
    ghost: &Ghost,
    place: Place,
    now: Instant,
) {
    let GhostLine {
        entry,
        row,
        ghost_row,
        top,
    } = ghost_line;
    let range = entry.shown_row(row);
    let still = |line: Line| Placed {
        top,
        line,
        clip: None,
    };

    if entry.kind == Kind::Prompt && row == 0 {
        let mut spans = pieces(entry, range.clone(), now);
        let heat = spans.first().map_or(0, |span| span.heat);
        let marker = Span::new(MARKER, MARKER_INK).heated(heat);
        match ghost.fate((ghost_row, 0), now) {
            Fate::Waiting => {
                spans.insert(0, marker);
                placed.push(still(Line::new(0, spans)));
                return;
            }
            Fate::Leaving(eased) => {
                placed.push(place.moving(top, Motion::Leaving(eased), Line::new(0, vec![marker])));
            }
            Fate::Gone => {}
        }
    }

    for (column, word) in words(&entry.text[range.clone()]) {
        let indent = indent_at(column);
        let start = range.start + word.start;
        match ghost.fate((ghost_row, column), now) {
            Fate::Waiting => {
                let rest = pieces(entry, start..range.end, now);
                placed.push(still(Line::new(indent, rest)));
                return;
            }
            Fate::Leaving(eased) => {
                let word = pieces(entry, start..range.start + word.end, now);
                placed.push(place.moving(top, Motion::Leaving(eased), Line::new(indent, word)));
            }
            Fate::Gone => {}
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/ghost.rs"]
mod tests;
