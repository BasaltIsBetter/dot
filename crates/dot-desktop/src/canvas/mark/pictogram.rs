use crate::canvas::layout::{Layout, TextSize};
use crate::canvas::mark::icon::Icon;
use crate::canvas::mark::shape::{Rectangle, Rgba};
use crate::canvas::scene::{Pane, Scene};
use dot_transcript::{Glyph, Placed};

const PICTOGRAMS: [(char, Icon); 7] = [
    ('▤', Icon::Tool(Glyph::Read)),
    ('⌕', Icon::Tool(Glyph::Search)),
    ('✎', Icon::Tool(Glyph::Edit)),
    ('✚', Icon::Tool(Glyph::Create)),
    ('⧉', Icon::Tool(Glyph::Agents)),
    ('↶', Icon::Back),
    ('↷', Icon::Forward),
];
const IN_THEIR_PLACE: char = ' ';

impl Scene {
    pub(crate) fn draw_pictograms(&mut self) {
        let layout = &self.layout;
        let mut drawn = Vec::new();
        for pane in &mut self.panes {
            let frame = pane_frame(pane);
            for placed in &mut pane.lines {
                drawn.extend(pictograms_of(placed, frame, layout));
            }
        }
        self.marks.extend(drawn);
    }
}

fn glyph_of(character: char) -> Option<Icon> {
    PICTOGRAMS
        .iter()
        .find(|(drawn, _)| *drawn == character)
        .map(|&(_, icon)| icon)
}

#[derive(Clone, Copy)]
struct PaneFrame {
    left: f32,
    size: TextSize,
    clip: [f32; 2],
}

const fn pane_frame(pane: &Pane) -> PaneFrame {
    PaneFrame {
        left: pane.left,
        size: pane.size,
        clip: [pane.top, pane.bottom],
    }
}

fn pictograms_of(placed: &mut Placed, pane: PaneFrame, layout: &Layout) -> Vec<Rectangle> {
    let has_any = placed.line.spans.iter().any(|span| {
        span.text
            .chars()
            .any(|character| glyph_of(character).is_some())
    });
    if !has_any {
        return Vec::new();
    }
    let advance = layout.advance_of(pane.size);
    let dot = layout.icon_dot();
    let top =
        (placed.top + (layout.line_height_of(pane.size) - Icon::height() * dot) / 2.0).round();
    let [clip_top, clip_bottom] = placed.clip.map_or(pane.clip, |[above, below]| {
        [above.max(pane.clip[0]), below.min(pane.clip[1])]
    });
    let alpha = placed.line.alpha;
    let mut column = u16::from(0_u8);
    let mut drawn = Vec::new();
    for span in &mut placed.line.spans {
        let color = Rgba::shaded(span.ink, span.heat, alpha);
        for character in span.text.chars() {
            if let Some(glyph) = glyph_of(character) {
                let left = (pane.left
                    + f32::from(placed.line.indent.saturating_add(column)) * advance)
                    .round();
                drawn.extend(glyph.drawn([left, top], dot, color).filter(|rectangle| {
                    rectangle.top >= clip_top && rectangle.top + rectangle.height <= clip_bottom
                }));
            }
            column = column.saturating_add(1);
        }
        if span
            .text
            .chars()
            .any(|character| glyph_of(character).is_some())
        {
            span.text = span
                .text
                .chars()
                .map(|character| {
                    if glyph_of(character).is_some() {
                        IN_THEIR_PLACE
                    } else {
                        character
                    }
                })
                .collect();
        }
    }
    drawn
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/mark/pictogram.rs"]
mod tests;
