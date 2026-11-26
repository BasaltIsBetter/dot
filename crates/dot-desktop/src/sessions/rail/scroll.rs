use std::time::{Duration, Instant};

use crate::app::Parts;
use crate::conversation::Stage;
use crate::event::Wheel;
use crate::sessions::rail::{
    BOTTOM, FOOT, FOOT_GAP, GAP, HEAD, NEW_CARD, PADDING_HIGH, TOP, to_points,
};
use crate::sessions::{SessionKey, Sessions};
use dot_transcript::to_f32;

const GESTURE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Slots {
    pub(crate) card: f32,
    pub(crate) pitch: f32,
    new_card: f32,
    top: f32,
    room: f32,
    content: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) first: f32,
    pub(crate) band: Option<[f32; 2]>,
    pub(crate) new_card: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RailScroll {
    offset: f32,
    shown: Option<SessionKey>,
    wheeling: Option<(bool, Instant)>,
}

impl Slots {
    pub(crate) fn of(stage: &Stage, count: usize) -> Self {
        let card = stage.points(PADDING_HIGH * 2.0 + HEAD + FOOT_GAP + FOOT);
        let gap = stage.points(GAP);
        let top = stage.points(TOP);
        Self {
            card,
            pitch: card + gap,
            new_card: stage.points(PADDING_HIGH * 2.0 + NEW_CARD),
            top,
            room: stage.height - stage.points(BOTTOM) - top,
            content: (to_points(count) * (card + gap) - gap).max(0.0),
        }
    }

    pub(crate) fn most(&self) -> f32 {
        (self.content - self.view()).max(0.0)
    }

    pub(crate) fn place(&self, offset: f32) -> Placement {
        if self.most() <= 0.0 {
            let first = (self.top + (self.view() - self.content) / 2.0).round();
            return Placement {
                first,
                band: None,
                new_card: [first + self.content + self.gap(), self.new_card],
            };
        }
        let band = [self.top.round(), (self.top + self.view()).round()];
        Placement {
            first: (self.top - offset.clamp(0.0, self.most())).round(),
            band: Some(band),
            new_card: [band[1] + self.gap(), self.new_card],
        }
    }

    fn reveal(&self, index: usize, offset: f32) -> f32 {
        let top = to_points(index) * self.pitch;
        offset.max(top + self.card - self.view()).min(top)
    }

    fn view(&self) -> f32 {
        (self.room - self.new_card - self.gap()).max(0.0)
    }

    fn gap(&self) -> f32 {
        self.pitch - self.card
    }
}

impl RailScroll {
    pub(crate) const fn offset(&self) -> f32 {
        self.offset
    }

    pub(crate) fn follow(&mut self, (key, index): (SessionKey, usize), slots: &Slots) {
        if self.shown != Some(key) {
            self.shown = Some(key);
            self.offset = slots.reveal(index, self.offset);
        }
        self.offset = self.offset.clamp(0.0, slots.most());
    }

    pub(crate) fn takes_the_wheel(&mut self, over_the_rail: bool, now: Instant) -> bool {
        let on_the_rail = match self.wheeling {
            Some((target, at)) if now.saturating_duration_since(at) < GESTURE => target,
            _ => over_the_rail,
        };
        self.wheeling = Some((on_the_rail, now));
        on_the_rail
    }

    pub(crate) fn scroll(&mut self, wheel: Wheel, slots: &Slots) {
        let down = match wheel {
            Wheel::Lines(lines) => -lines * slots.pitch,
            Wheel::Pixels(pixels) => -to_f32(pixels),
        };
        self.offset = (self.offset + down).clamp(0.0, slots.most());
    }
}

impl<P: Parts> Sessions<P> {
    pub(crate) const fn rail_offset(&self) -> f32 {
        self.rail.offset()
    }

    pub(crate) fn follow_rail(&mut self, stage: &Stage) {
        let slots = Slots::of(stage, self.opened.len());
        self.rail.follow((self.active().key, self.active), &slots);
    }

    pub(crate) fn wheel_rail(
        &mut self,
        wheel: Wheel,
        at: [f32; 2],
        (stage, now): (&Stage, Instant),
    ) -> bool {
        if stage.rail.is_none() {
            return false;
        }
        if !self.rail.takes_the_wheel(is_over_the_rail(stage, at), now) {
            return false;
        }
        let slots = Slots::of(stage, self.opened.len());
        self.rail.follow((self.active().key, self.active), &slots);
        self.rail.scroll(wheel, &slots);
        true
    }
}

fn is_over_the_rail(stage: &Stage, [x, y]: [f32; 2]) -> bool {
    stage
        .rail
        .is_some_and(|[left, right]| x >= left && x < right && y < stage.floor)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/scroll.rs"]
mod tests;
