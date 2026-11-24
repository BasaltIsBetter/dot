mod face;
mod mark;
mod name;
mod palette;

use std::ops::Range;
use std::time::Instant;

use dot_api::{Hue, Translator};

use crate::canvas::{Layout, Pane, Rectangle};
use crate::conversation::Stage;
use crate::sessions::card::Card;
use dot_transcript::Ink;

use self::mark::{corner_colour, corners};

const TOP: f32 = 24.0;
const BOTTOM: f32 = 64.0;
const GAP: f32 = 12.0;
const PADDING_HIGH: f32 = 9.0;
const PADDING_WIDE: f32 = 13.0;
const HEAD: f32 = 16.0;
const FOOT_GAP: f32 = 6.0;
const FOOT: f32 = 12.0;
const NEW_CARD: f32 = 12.0;
const CORNER: f32 = 9.0;
const HERE_CORNER: f32 = 12.0;
const QUIET: Ink = Ink::Secondary;
const PALETTE_GAP: f32 = 4.0;
const PALETTE_ROW: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Switch(usize),
    Close(usize),
    Open,
    Hue(usize),
    Pick(usize, Hue),
    Name(usize),
}

#[derive(Debug, Default)]
pub(crate) struct Rail {
    pub(crate) panes: Vec<Pane>,
    pub(crate) marks: Vec<Rectangle>,
    hits: Vec<([f32; 4], Hit)>,
}

struct Frame<'a> {
    stage: &'a Stage,
    layout: &'a Layout,
    left: f32,
    right: f32,
    hover: Option<[f32; 2]>,
    now: Instant,
}

impl Rail {
    pub(crate) fn of(
        cards: &[Card],
        (stage, layout): (&Stage, &Layout),
        hover: Option<[f32; 2]>,
        now: Instant,
        translator: &impl Translator,
    ) -> Option<Self> {
        let [left, right] = stage.rail?;
        let frame = Frame {
            stage,
            layout,
            left,
            right,
            hover,
            now,
        };
        let heights: Vec<f32> = cards.iter().map(|card| frame.height_of(card)).collect();
        let gap = stage.points(GAP);
        let new_height = stage.points(PADDING_HIGH * 2.0 + NEW_CARD);
        let top = stage.points(TOP);
        let room = stage.height - stage.points(BOTTOM) - top;
        let here = cards.iter().position(|card| card.here).unwrap_or(0);
        let shown = in_sight(&heights, gap, room - new_height, here);
        let total: f32 =
            heights[shown.clone()].iter().sum::<f32>() + new_height + gap * to_points(shown.len());
        let mut y = (top + ((room - total) / 2.0).max(0.0)).round();

        let mut rail = Self::default();
        let closable = cards.len() > 1;
        for index in shown {
            let height = heights[index];
            rail.card(
                &frame,
                (index, &cards[index]),
                [y, height],
                (closable, translator),
            );
            y += height + gap;
        }
        rail.new_card(&frame, [y, new_height]);
        Some(rail)
    }

    pub(crate) fn hit(&self, [x, y]: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|([left, top, width, height], _)| {
                x >= *left && x < left + width && y >= *top && y < top + height
            })
            .map(|(_, hit)| *hit)
    }

    #[cfg(test)]
    pub(crate) fn spot_of(&self, wanted: Hit) -> Option<[f32; 2]> {
        self.hits
            .iter()
            .find(|(_, hit)| *hit == wanted)
            .map(|([left, top, width, height], _)| [left + width / 2.0, top + height / 2.0])
    }

    fn card(
        &mut self,
        frame: &Frame<'_>,
        (index, card): (usize, &Card),
        [top, height]: [f32; 2],
        (closable, translator): (bool, &impl Translator),
    ) {
        let bounds = [frame.left, top, frame.right - frame.left, height];
        let hovered = frame.is_over(bounds);
        self.hits.push((bounds, Hit::Switch(index)));
        let arm = frame
            .stage
            .points(if card.here { HERE_CORNER } else { CORNER });
        let colour = corner_colour(card, hovered, frame.now);
        self.marks.extend(corners(bounds, arm, colour));

        let head_top = top + frame.stage.points(PADDING_HIGH);
        self.head(frame, (index, card), head_top, (closable, hovered));
        let mut above_the_foot = HEAD + FOOT_GAP;
        if card.picking {
            let palette_top = head_top + frame.stage.points(HEAD + PALETTE_GAP);
            self.palette(frame, (index, card), palette_top);
            above_the_foot += PALETTE_GAP + PALETTE_ROW;
        }
        let foot_top = head_top + frame.stage.points(above_the_foot);
        self.foot(frame, card, foot_top, translator);
    }
}

impl Frame<'_> {
    fn height_of(&self, card: &Card) -> f32 {
        let palette = if card.picking {
            PALETTE_GAP + PALETTE_ROW
        } else {
            0.0
        };
        self.stage
            .points(PADDING_HIGH * 2.0 + HEAD + palette + FOOT_GAP + FOOT)
    }

    fn is_over(&self, [left, top, width, height]: [f32; 4]) -> bool {
        self.hover
            .is_some_and(|[x, y]| x >= left && x < left + width && y >= top && y < top + height)
    }

    fn inner(&self) -> [f32; 2] {
        let padding = self.stage.points(PADDING_WIDE);
        [self.left + padding, self.right - padding]
    }
}

fn to_points(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a room in columns is floored and non-negative, far below usize::MAX"
)]
fn to_columns(room: f32) -> usize {
    room.floor() as usize
}

fn in_sight(heights: &[f32], gap: f32, room: f32, here: usize) -> Range<usize> {
    let Some(height) = heights.get(here) else {
        return 0..heights.len();
    };
    let mut used = height + gap;
    let (mut start, mut end) = (here, here + 1);
    while start > 0 && used + heights[start - 1] + gap <= room {
        start -= 1;
        used += heights[start] + gap;
    }
    while end < heights.len() && used + heights[end] + gap <= room {
        used += heights[end] + gap;
        end += 1;
    }
    start..end
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/mod.rs"]
mod tests;
