use crate::canvas::layout::{Layout, TextSize, whole_columns};

const STRIP_HEIGHT: f32 = 28.0;
const STRIP_PADDING: f32 = 16.0;
const GAUGE_WIDTH: f32 = 50.0;
const GAUGE_HEIGHT: f32 = 5.0;
const ZONE_RULE_HEIGHT: f32 = 14.0;
const MIDDLE_ZONE_FROM: f32 = 640.0;

impl Layout {
    pub(crate) fn strip_top(&self) -> f32 {
        (self.height - STRIP_HEIGHT * self.scale).max(0.0).floor()
    }

    pub(crate) fn strip_middle(&self) -> f32 {
        self.strip_top() + (STRIP_HEIGHT * self.scale / 2.0).floor()
    }

    pub(crate) fn strip_text_top(&self) -> f32 {
        (self.strip_middle() - self.line_height_of(TextSize::Small) / 2.0).floor()
    }

    pub(crate) fn strip_x(&self, column: u16) -> f32 {
        self.strip_padding() + f32::from(column) * self.advance_of(TextSize::Small)
    }

    pub(crate) fn strip_columns(&self) -> u16 {
        whole_columns((self.width - 2.0 * self.strip_padding()) / self.advance_of(TextSize::Small))
    }

    pub(crate) fn has_room_for_the_middle_zone(&self) -> bool {
        self.width > MIDDLE_ZONE_FROM * self.scale
    }

    pub(crate) fn gauge_size(&self) -> [f32; 2] {
        [
            (GAUGE_WIDTH * self.scale).round().max(1.0),
            (GAUGE_HEIGHT * self.scale).round().max(1.0),
        ]
    }

    pub(crate) fn zone_rule_height(&self) -> f32 {
        (ZONE_RULE_HEIGHT * self.scale).round().max(1.0)
    }

    fn strip_padding(&self) -> f32 {
        (STRIP_PADDING * self.scale).floor()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/layout/strip.rs"]
mod tests;
