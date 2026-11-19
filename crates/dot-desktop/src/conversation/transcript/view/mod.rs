mod ghost;

use std::ops::Range;
use std::time::Instant;

use crate::canvas::{Ink, Layout, Line, Placed, SETTLE, Span, columns, to_f32};
use crate::conversation::scroll::rows;
use crate::conversation::transcript::entry::{Arrival, Entry, Kind};
use crate::conversation::transcript::ghost::Ghost;
use crate::conversation::transcript::history::{Row, Transcript};
use crate::conversation::zone::Band;
use crate::conversation::{MARKER, TEXT_INDENT};

const CARET: &str = "_";
const CARET_INK: Ink = Ink::Live;
const MARKER_INK: Ink = Ink::Receding;

#[derive(Clone, Copy, Debug)]
struct Place {
    band: Band,
    line_height: f32,
    first: usize,
    offset: f64,
}

#[derive(Clone, Copy, Debug)]
enum Motion {
    Rising(f64),
    Leaving(f64),
}

#[derive(Clone, Copy)]
struct TextRow<'a> {
    entry: &'a Entry,
    row: usize,
    top: f32,
    rolling: Option<&'a Ghost>,
    caret: bool,
}

pub(crate) fn lines(
    transcript: &Transcript,
    band: Band,
    layout: &Layout,
    now: Instant,
) -> Vec<Placed> {
    let line_height = layout.line_height();
    let window = transcript.window(band.rows(line_height), now);
    let place = Place {
        band,
        line_height,
        first: window.first,
        offset: window.offset,
    };
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
        let text_row = TextRow {
            entry,
            row,
            top: place.top_at(down),
            rolling: transcript.ghost().filter(|ghost| ghost.slot() == position),
            caret: transcript.is_live() && transcript.is_last_row(index),
        };
        push_row(&mut placed, text_row, place, now);
    }
    if let Some(ghost) = transcript.ghost() {
        ghost::push_ghost(&mut placed, transcript, ghost, place, now);
    }
    placed
}

impl Place {
    fn top_at(self, down: usize) -> f32 {
        self.band
            .row_top(self.offset + rows(down), self.line_height)
    }

    fn top_of(self, index: usize) -> Option<f32> {
        index.checked_sub(self.first).map(|down| self.top_at(down))
    }

    fn moving(self, top: f32, motion: Motion, line: Line) -> Placed {
        let shift = match motion {
            Motion::Rising(eased) => 1.0 - eased,
            Motion::Leaving(eased) => -eased,
        };
        Placed {
            top: (top + to_f32(shift) * self.line_height).round(),
            line,
            clip: Some([top, top + self.line_height]),
        }
    }
}

fn push_row(placed: &mut Vec<Placed>, text_row: TextRow<'_>, place: Place, now: Instant) {
    let TextRow {
        entry,
        row,
        top,
        rolling,
        caret,
    } = text_row;
    let ink = ink_of(entry.kind);
    let range = entry.row(row);
    let first = placed.len();
    let rise_of = |arrival: Option<&Arrival>| {
        let arrival =
            arrival.filter(|arrival| rolling.is_some_and(|ghost| ghost.rolls(arrival.start)))?;
        SETTLE.eased(now.saturating_duration_since(arrival.at))
    };
    let mut column = 0;

    entry.each_piece_by(
        range.clone(),
        |arrival| (entry.heat(arrival, now), rise_of(arrival)),
        |piece, (heat, rise)| {
            let text = &entry.text[piece];
            let span = Span::new(text, ink).heated(heat);
            let indent = indent_at(column);
            column += columns(text);
            if let Some(rise) = rise {
                placed.push(place.moving(top, Motion::Rising(rise), Line::new(indent, vec![span])));
                return;
            }
            match placed[first..]
                .last_mut()
                .filter(|last| last.clip.is_none())
            {
                Some(last) => last.line.spans.push(span),
                None => placed.push(Placed {
                    top,
                    line: Line::new(indent, vec![span]),
                    clip: None,
                }),
            }
        },
    );

    if entry.kind == Kind::Prompt && row == 0 {
        let line = &mut placed[first].line;
        let heat = line.spans.first().map_or(0, |span| span.heat);
        line.spans
            .insert(0, Span::new(MARKER, MARKER_INK).heated(heat));
        line.indent = 0;
    }
    if caret && let Some(last) = placed[first..].last_mut() {
        last.line.spans.push(Span::new(CARET, CARET_INK));
    }
}

fn pieces(entry: &Entry, bytes: Range<usize>, now: Instant) -> Vec<Span> {
    let ink = ink_of(entry.kind);
    let mut spans = Vec::new();
    entry.each_piece(bytes, now, |piece, heat| {
        spans.push(Span::new(&entry.text[piece], ink).heated(heat));
    });
    spans
}

fn indent_at(column: usize) -> u16 {
    TEXT_INDENT.saturating_add(u16::try_from(column).unwrap_or(u16::MAX))
}

const fn ink_of(kind: Kind) -> Ink {
    match kind {
        Kind::Prompt | Kind::Answer => Ink::Full,
        Kind::Thinking | Kind::Attachment | Kind::Tool => Ink::Secondary,
        Kind::Notice => Ink::Receding,
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/transcript/view/mod.rs"]
mod tests;
