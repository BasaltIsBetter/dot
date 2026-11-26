use crate::canvas::{Paint, Rectangle, TextSize};
use crate::sessions::card::Card;
use crate::sessions::rail::{Frame, HEAD, Hit, QUIET, Rail, to_points};
use dot_transcript::{Ink, Span, columns, truncated};

const UNDERLINE_BELOW: f32 = 3.0;
const HAIRLINE: f32 = 1.0;
const WASH: u8 = 97;

impl Rail {
    pub(super) fn name(
        &mut self,
        frame: &Frame<'_>,
        (index, card): (usize, &Card),
        [left, top, width]: [f32; 3],
        (room, hovered): (usize, bool),
    ) {
        let stage = frame.stage;
        let field_paint = Paint::ink(card.colour);
        let small = frame.layout.advance_of(TextSize::Small);
        let underline_top = (frame
            .layout
            .text_top(TextSize::Small, top, stage.points(HEAD))
            + frame.layout.line_height_of(TextSize::Small)
            - stage.points(UNDERLINE_BELOW))
        .round();
        if let Some(field) = &card.field {
            let shown = truncated(&field.text, room);
            let shown_width = to_points(columns(&shown)) * small;
            if field.whole && !shown.is_empty() {
                self.marks.push(Rectangle {
                    left: left.round(),
                    top: top.round(),
                    width: shown_width.round(),
                    height: stage.points(HEAD),
                    color: Paint::shaded(card.colour, 0, WASH),
                });
            }
            self.text(
                frame,
                [left, top],
                TextSize::Small,
                Span::new(shown, Ink::Full),
            );
            if !field.whole {
                let caret = left + to_points(field.caret.min(room)) * small;
                self.marks.push(Rectangle {
                    left: caret.round(),
                    top: top.round(),
                    width: HAIRLINE,
                    height: stage.points(HEAD),
                    color: field_paint,
                });
            }
            self.marks
                .push(underline([left, underline_top, width], field_paint));
            self.hits
                .push(([left, top, width, stage.points(HEAD)], Hit::Name(index)));
            return;
        }
        let shown = truncated(&card.name, room);
        let shown_width = to_points(columns(&shown)) * small;
        let ink = match card.name_tint {
            Some(tint) => Ink::Rgb(tint),
            None if card.here || hovered => Ink::Full,
            None => QUIET,
        };
        let bounds = [left, top, shown_width, stage.points(HEAD)];
        if frame.is_over(bounds) {
            self.marks.push(underline(
                [left, underline_top, shown_width],
                Paint::ink(Ink::Receding),
            ));
        }
        self.hits.push((bounds, Hit::Name(index)));
        self.text(frame, [left, top], TextSize::Small, Span::new(shown, ink));
    }
}

fn underline([left, top, width]: [f32; 3], color: Paint) -> Rectangle {
    Rectangle {
        left: left.round(),
        top,
        width: width.round(),
        height: HAIRLINE,
        color,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/name.rs"]
mod tests;
