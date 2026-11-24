mod arrow;
mod fine;
mod status;
mod tool;

use dot_transcript::Glyph;

pub(crate) use self::fine::Fine;

use crate::canvas::layout::{Layout, TextSize};
use crate::canvas::mark::shape::{Paint, Rectangle};

const WIDE: u8 = 9;
const HIGH: u8 = 7;
const CELLS: f32 = 2.0;
const DOT: u8 = b'#';

type Dots = [&'static str; HIGH as usize];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    Model,
    Tool(Glyph),
    Back,
    Forward,
}

impl Icon {
    pub(crate) fn height() -> f32 {
        f32::from(HIGH)
    }

    pub(crate) fn in_the_strip(
        self,
        layout: &Layout,
        column: u16,
        color: Paint,
    ) -> impl Iterator<Item = Rectangle> {
        let dot = layout.icon_dot();
        let width = dot * f32::from(WIDE);
        let height = dot * f32::from(HIGH);
        let cells = CELLS * layout.advance_of(TextSize::Small);
        let left = (layout.strip_x(column) + (cells - width) / 2.0).round();
        let top = (layout.strip_middle() - height / 2.0).round();

        self.drawn([left, top], dot, color)
    }

    pub(crate) fn drawn(
        self,
        [left, top]: [f32; 2],
        dot: f32,
        color: Paint,
    ) -> impl Iterator<Item = Rectangle> {
        self.rows()
            .into_iter()
            .zip(0..HIGH)
            .flat_map(move |(row, row_index)| {
                runs(row).map(move |(start, length)| Rectangle {
                    left: left + f32::from(start) * dot,
                    top: top + f32::from(row_index) * dot,
                    width: f32::from(length) * dot,
                    height: dot,
                    color,
                })
            })
    }

    const fn rows(self) -> Dots {
        match self {
            Self::Model => status::ROBOT,
            Self::Tool(glyph) => tool::tool(glyph),
            Self::Back => arrow::BACK,
            Self::Forward => arrow::FORWARD,
        }
    }
}

fn runs(row: &'static str) -> impl Iterator<Item = (u8, u8)> {
    let filled = |column: u8| row.as_bytes().get(usize::from(column)) == Some(&DOT);
    (0..WIDE).filter_map(move |column| {
        let starts = filled(column) && (column == 0 || !filled(column - 1));
        if !starts {
            return None;
        }
        let length = (column..WIDE).take_while(|&next| filled(next)).count();
        u8::try_from(length).ok().map(|length| (column, length))
    })
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/canvas/mark/icon/mod.rs"]
mod tests;
