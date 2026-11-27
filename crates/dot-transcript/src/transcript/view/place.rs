use crate::line::Band;
use crate::line::{Line, Placed};
use crate::reading::rows;
use crate::reading::to_f32;

const TRAVEL: f64 = 0.7;

#[derive(Clone, Copy, Debug)]
pub(super) struct Place {
    pub(super) band: Band,
    pub(super) line_height: f32,
    pub(super) first: usize,
    pub(super) offset: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Motion {
    Rising(f64),
    Leaving(f64),
}

impl Place {
    pub(super) fn top_at(self, down: usize) -> f32 {
        self.band
            .row_top(self.offset + rows(down), self.line_height)
    }

    pub(super) fn top_of(self, index: usize) -> Option<f32> {
        index.checked_sub(self.first).map(|down| self.top_at(down))
    }

    pub(super) fn moving(self, top: f32, motion: Motion, line: Line) -> Placed {
        let (shift, shown) = match motion {
            Motion::Rising(eased) => (1.0 - eased, eased),
            Motion::Leaving(eased) => (-eased, 1.0 - eased),
        };
        Placed {
            top: (top + to_f32(shift * TRAVEL) * self.line_height).round(),
            line: line.faded(shown),
            clip: None,
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/view/place.rs"]
mod tests;
