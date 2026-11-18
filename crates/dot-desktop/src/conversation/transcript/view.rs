use std::time::Instant;

use crate::canvas::{Ink, Layout, Line, Placed, Span};
use crate::conversation::scroll::rows;
use crate::conversation::transcript::fade::heat;
use crate::conversation::transcript::history::{Entry, Row, Transcript};
use crate::conversation::zone::Band;
use crate::conversation::{MARKER, TEXT_INDENT};

pub(crate) fn lines(
    transcript: &Transcript,
    band: Band,
    layout: &Layout,
    now: Instant,
) -> Vec<Placed> {
    let line_height = layout.line_height();
    let window = transcript.window(band.rows(line_height));

    (window.first..window.end)
        .enumerate()
        .filter_map(|(down, index)| {
            let Row::Text { entry, row } = transcript.row(index)? else {
                return None;
            };
            Some(Placed {
                top: band.row_top(window.offset + rows(down), line_height),
                line: line(entry, row, now),
            })
        })
        .collect()
}

fn line(entry: &Entry, row: usize, now: Instant) -> Line {
    let heat = heat(now.saturating_duration_since(entry.arrived_at));
    let text = Span::new(&entry.text[entry.rows[row].clone()], Ink::Full).heated(heat);
    if row == 0 {
        let marker = Span::new(MARKER, Ink::Receding).heated(heat);
        return Line::new(0, vec![marker, text]);
    }
    Line::new(TEXT_INDENT, vec![text])
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/transcript/view.rs"]
mod tests;
