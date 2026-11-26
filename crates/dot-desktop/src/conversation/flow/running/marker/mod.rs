mod draw;

use std::time::{Duration, Instant};

use dot_api::Translator;

use self::draw::placement;
use crate::canvas::{Layout, Silhouette};
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::name_of;
use crate::conversation::flow::running::RunningRow;
use crate::conversation::stage::Stage;
use dot_transcript::{Glide, Motion, rows};

const MARGIN: f32 = 8.0;
const FADE: Motion = Motion::landing(Duration::from_millis(300));
const NAMES_SEPARATOR: &str = ", ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Above,
    Below,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Sighting {
    side: Side,
    row: usize,
    count: u64,
    names: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::conversation::flow) struct Marker {
    shown: Glide,
    last: Option<Sighting>,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            shown: Glide::at_rest(0.0, FADE),
            last: None,
        }
    }
}

impl Marker {
    pub(in crate::conversation::flow) fn is_fading(&self, now: Instant) -> bool {
        self.shown.is_moving(now)
    }

    fn see(&mut self, sighting: Option<Sighting>, now: Instant) {
        let target = if sighting.is_some() { 1.0 } else { 0.0 };
        self.shown.toward(target, now);
        if sighting.is_some() {
            self.last = sighting;
        }
    }
}

impl Conversation {
    pub(in crate::conversation::flow) fn watch_lanes(&mut self, stage: &Stage, now: Instant) {
        let sighting = self.lanes_out_of_sight(stage, now);
        self.marker.see(sighting, now);
    }

    pub(crate) fn press_marker(
        &mut self,
        at: [f32; 2],
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) -> bool {
        if self.agent_view.is_some() || self.marker.shown.target() < 1.0 {
            return false;
        }
        let Some(sighting) = self.marker.last.as_ref() else {
            return false;
        };
        let stage = Stage::of(layout);
        let figure = Silhouette::size_at(layout.scale());
        if !placement(sighting, (&stage, layout), figure, translator).holds(at) {
            return false;
        }
        let row = rows(sighting.row);
        self.reader.scroll_by(row - self.reader.focus(now));
        self.help.stir(now);
        true
    }

    fn lanes_out_of_sight(&self, stage: &Stage, now: Instant) -> Option<Sighting> {
        self.agents.working().next()?;
        let first = self.below(now).running?;
        let top_row = stage.top_row(self.reader.focus(now));
        let mut above = Vec::new();
        let mut below = Vec::new();
        for (at, running) in self.running_rows(now).into_iter().enumerate() {
            let RunningRow::Lane(_, agent) = running else {
                continue;
            };
            let row = first + at;
            match side_of(stage.row_top(row, top_row), stage) {
                Some(Side::Above) => above.push((row, name_of(agent))),
                Some(Side::Below) => below.push((row, name_of(agent))),
                None => {}
            }
        }
        let (side, lanes) = match (above.is_empty(), below.is_empty()) {
            (false, _) => (Side::Above, above),
            (true, false) => (Side::Below, below),
            (true, true) => return None,
        };
        let names: Vec<&str> = lanes.iter().map(|&(_, name)| name).collect();
        Some(Sighting {
            side,
            row: lanes[0].0,
            count: u64::try_from(lanes.len()).unwrap_or(u64::MAX),
            names: names.join(NAMES_SEPARATOR),
        })
    }
}

fn side_of(top: f32, stage: &Stage) -> Option<Side> {
    if top < stage.points(MARGIN) {
        return Some(Side::Above);
    }
    (top + stage.row > stage.lowest_row_bottom()).then_some(Side::Below)
}

#[cfg(test)]
#[path = "../../../../../../../tests/dot-desktop/conversation/flow/running/marker/mod.rs"]
mod tests;
