use dot_api::Hue;

use crate::canvas::{Paint, Rectangle};
use crate::sessions::card::Card;
use crate::sessions::rail::{Frame, Hit, PALETTE_ROW, Rail, to_points};
use dot_transcript::Ink;

const SWATCH: f32 = 9.0;
const SWATCH_GAP: f32 = 5.0;
const RING_OFFSET: f32 = 2.0;
const HAIRLINE: f32 = 1.0;
const RESTING: u8 = 128;
const HOVERED: u8 = 217;

impl Rail {
    pub(super) fn palette(&mut self, frame: &Frame<'_>, (index, card): (usize, &Card), top: f32) {
        let stage = frame.stage;
        let [left, _] = frame.inner();
        let side = stage.points(SWATCH);
        let gap = stage.points(SWATCH_GAP);
        let swatch_top = (top + (stage.points(PALETTE_ROW) - side) / 2.0).round();
        for (place, hue) in Hue::ALL.into_iter().enumerate() {
            let x = (left + to_points(place) * (side + gap)).round();
            let bounds = [x, swatch_top, side, side];
            let current = card.hue == Some(hue);
            let alpha = if current {
                u8::MAX
            } else if frame.is_over(bounds) {
                HOVERED
            } else {
                RESTING
            };
            self.marks.push(Rectangle {
                left: x,
                top: swatch_top,
                width: side,
                height: side,
                color: Paint {
                    alpha,
                    ..Paint::ink(Ink::Rgb(hue.rgb()))
                },
            });
            if current {
                self.marks.extend(ring(
                    bounds,
                    stage.points(RING_OFFSET),
                    Paint::ink(Ink::Full),
                ));
            }
            let reach = [x - gap / 2.0, top, side + gap, stage.points(PALETTE_ROW)];
            self.hits.push((reach, Hit::Pick(index, hue)));
        }
    }
}

fn ring([left, top, side, _]: [f32; 4], offset: f32, color: Paint) -> [Rectangle; 4] {
    let outer = side + 2.0 * offset;
    let [left, top] = [left - offset, top - offset];
    let rectangle = |left: f32, top: f32, width: f32, height: f32| Rectangle {
        left,
        top,
        width,
        height,
        color,
    };
    [
        rectangle(left, top, outer, HAIRLINE),
        rectangle(left, top + outer - HAIRLINE, outer, HAIRLINE),
        rectangle(left, top, HAIRLINE, outer),
        rectangle(left + outer - HAIRLINE, top, HAIRLINE, outer),
    ]
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/palette.rs"]
mod tests;
