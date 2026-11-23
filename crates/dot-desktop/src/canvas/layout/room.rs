const WIDE_FROM: f32 = 900.0;
const RAILED_FROM: f32 = 1_100.0;
const NARROW_RESERVE: f32 = 48.0;
const WIDE_RESERVE: f32 = 190.0;
const RAILED_LEFT_RESERVE: f32 = 200.0;
const RAILED_RIGHT_RESERVE: f32 = 440.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Room {
    #[default]
    Narrow,
    Wide,
    Railed,
}

impl Room {
    pub(crate) fn of(width_in_points: f32) -> Self {
        if width_in_points >= RAILED_FROM {
            Self::Railed
        } else if width_in_points >= WIDE_FROM {
            Self::Wide
        } else {
            Self::Narrow
        }
    }

    pub(crate) const fn reserves(self) -> [f32; 2] {
        match self {
            Self::Narrow => [NARROW_RESERVE; 2],
            Self::Wide => [WIDE_RESERVE; 2],
            Self::Railed => [RAILED_LEFT_RESERVE, RAILED_RIGHT_RESERVE],
        }
    }

    pub(crate) const fn has_gutter(self) -> bool {
        !matches!(self, Self::Narrow)
    }

    pub(crate) const fn has_rail(self) -> bool {
        matches!(self, Self::Railed)
    }
}

pub(crate) fn shift(free: f32, [left, right]: [f32; 2]) -> f32 {
    (free * left / (left + right) - free / 2.0).min(0.0)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/layout/room.rs"]
mod tests;
