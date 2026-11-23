use crate::canvas::layout::{ADVANCE_PER_SIZE, Layout};

const SMALL_TEXT_SIZE: f32 = 11.0;
const SMALL_LINE_HEIGHT: f32 = 18.0;
const TINY_TEXT_SIZE: f32 = 9.0;
const TINY_LINE_HEIGHT: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum TextSize {
    #[default]
    Regular,
    Small,
    Tiny,
}

impl Layout {
    pub(crate) fn font_size_of(&self, size: TextSize) -> f32 {
        match size {
            TextSize::Regular => self.font_size(),
            TextSize::Small => SMALL_TEXT_SIZE * self.scale,
            TextSize::Tiny => TINY_TEXT_SIZE * self.scale,
        }
    }

    pub(crate) fn line_height_of(&self, size: TextSize) -> f32 {
        match size {
            TextSize::Regular => self.line_height(),
            TextSize::Small => SMALL_LINE_HEIGHT * self.scale,
            TextSize::Tiny => TINY_LINE_HEIGHT * self.scale,
        }
    }

    pub(crate) fn advance_of(&self, size: TextSize) -> f32 {
        self.font_size_of(size) * ADVANCE_PER_SIZE
    }

    pub(crate) fn text_top(&self, size: TextSize, top: f32, height: f32) -> f32 {
        (top + (height - self.line_height_of(size)) / 2.0).round()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/layout/text.rs"]
mod tests;
