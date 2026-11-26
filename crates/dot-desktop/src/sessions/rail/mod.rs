mod face;
mod mark;
mod name;
mod scroll;

use std::time::Instant;

use dot_api::Translator;

use crate::canvas::{Layout, Pane, Rectangle, clip_span, corners};
use crate::conversation::Stage;
use crate::sessions::card::Card;
use dot_transcript::Ink;

use self::mark::corner_colour;
use self::scroll::Slots;

pub(crate) use self::scroll::RailScroll;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Switch(usize),
    Close(usize),
    Open,
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
    clip: Option<[f32; 2]>,
    now: Instant,
}

impl Rail {
    pub(crate) fn of(
        cards: &[Card],
        (stage, layout): (&Stage, &Layout),
        (hover, offset): (Option<[f32; 2]>, f32),
        now: Instant,
        translator: &impl Translator,
    ) -> Option<Self> {
        let [left, right] = stage.rail?;
        let slots = Slots::of(stage, cards.len());
        let placement = slots.place(offset);
        let mut frame = Frame {
            stage,
            layout,
            left,
            right,
            hover,
            clip: placement.band,
            now,
        };

        let mut rail = Self::default();
        let closable = cards.len() > 1;
        for (index, card) in cards.iter().enumerate() {
            let top = placement.first + to_points(index) * slots.pitch;
            if frame.hides([top, slots.card]) {
                continue;
            }
            rail.card(
                &frame,
                (index, card),
                [top, slots.card],
                (closable, translator),
            );
        }
        if let Some(band) = placement.band {
            rail.clip(band);
        }

        frame.clip = None;
        rail.new_card(&frame, placement.new_card);
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
        let foot_top = head_top + frame.stage.points(HEAD + FOOT_GAP);
        self.foot(frame, card, foot_top, translator);
    }

    fn clip(&mut self, band: [f32; 2]) {
        self.marks.retain_mut(|mark| mark.clip_to(band));
        self.hits
            .retain_mut(|([_, top, _, height], _)| clip_span([top, height], band));
        let [low, high] = band;
        for pane in &mut self.panes {
            pane.top = pane.top.max(low);
            pane.bottom = pane.bottom.min(high);
        }
    }
}

impl Frame<'_> {
    fn hides(&self, [top, height]: [f32; 2]) -> bool {
        self.clip
            .is_some_and(|[low, high]| top + height <= low || top >= high)
    }

    fn is_over(&self, [left, top, width, height]: [f32; 4]) -> bool {
        let shown = |y: f32| self.clip.is_none_or(|[low, high]| y >= low && y < high);
        self.hover.is_some_and(|[x, y]| {
            x >= left && x < left + width && y >= top && y < top + height && shown(y)
        })
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

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/mod.rs"]
mod tests;
