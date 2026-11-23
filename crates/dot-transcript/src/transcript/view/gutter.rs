use std::time::Instant;

use crate::ink::Ink;
use crate::line::{Line, Span, columns, truncated};
use crate::reading::rows;
use crate::transcript::history::Transcript;
use crate::transcript::tool::Glyph;
use crate::transcript::view::{View, place, row_alpha, window};

const TEXT_INK: Ink = Ink::Secondary;
const GLYPH_COLUMNS: u16 = 3;
const RESULT_GAP: &str = "  ";

/// A tool placed in the gutter beside the reply, on the row where Claude called it.
#[derive(Clone, Debug, PartialEq)]
pub struct Marked {
    /// Top of its row, in pixels.
    pub top: f32,
    /// What the tool does.
    pub glyph: Glyph,
    /// How orange the tool is: all of it while it runs, cooling once it ended.
    pub heat: u8,
    /// Its label then its result, from the columns the glyph leaves, faded like its row.
    pub line: Line,
}

/// Returns the tools `view` shows in the gutter of `transcript`, at `now`, each on the row
/// where Claude called it, or below the tool before it on that row; `columns` wide, glyph
/// included.
#[must_use]
pub fn gutter(transcript: &Transcript, view: &View, columns: usize, now: Instant) -> Vec<Marked> {
    let window = window(transcript, view);
    let place = place(view, window);
    let active_from = transcript.active_from();
    let room = columns.saturating_sub(usize::from(GLYPH_COLUMNS));
    let past_the_band = view.top + view.band.rows(view.line_height);
    let mut marked = Vec::new();
    let mut next_free = 0;

    for (row, entry) in transcript.tools_from(window.first) {
        let slot = row.max(next_free);
        next_free = slot + 1;
        if slot < window.first {
            continue;
        }
        if rows(slot) >= past_the_band {
            break;
        }
        let Some(mark) = entry.mark.as_deref() else {
            continue;
        };
        let heat = entry.heat(None, now);
        let text = labelled(&mark.tool.label, mark.result.as_deref(), room);
        let alpha = row_alpha(row, active_from, view.review);
        marked.push(Marked {
            top: place.top_of_any(slot),
            glyph: mark.tool.glyph,
            heat,
            line: Line::new(GLYPH_COLUMNS, vec![Span::new(text, TEXT_INK).heated(heat)])
                .faded(alpha),
        });
    }
    marked
}

fn labelled(label: &str, result: Option<&str>, room: usize) -> String {
    let Some(result) = result.filter(|result| !result.is_empty()) else {
        return truncated(label, room);
    };
    let kept = room.saturating_sub(columns(result) + columns(RESULT_GAP));
    format!("{}{RESULT_GAP}{result}", truncated(label, kept))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/gutter.rs"]
mod tests;
