use dot_api::Effort;

use crate::canvas::layout::Layout;
use crate::canvas::line::Ink;
use crate::canvas::mark::shape::Rectangle;

const WIDE: u8 = 9;
const HIGH: u8 = 7;
const CELLS: f32 = 2.0;
const DOT: u8 = b'#';
const ICON_INK: Ink = Ink::Receding;
const SPARK: [&str; 7] = [
    "....#....",
    "....#....",
    "...###...",
    ".#######.",
    "...###...",
    "....#....",
    "....#....",
];
const BRANCH: [&str; 7] = [
    ".##...##.",
    ".##...##.",
    "..#...#..",
    "..#..#...",
    "..###....",
    "..#......",
    ".##......",
];
const LINES: [&str; 7] = [
    ".#####...",
    ".........",
    ".######..",
    ".........",
    ".####....",
    ".........",
    ".#####...",
];
const BATTERY: [&str; 7] = [
    ".........",
    "########.",
    "#......##",
    "#......##",
    "#......##",
    "########.",
    ".........",
];
const DOLLAR: [&str; 7] = [
    "....#....",
    "..#####..",
    "..#.#....",
    "..#####..",
    "....#.#..",
    "..#####..",
    "....#....",
];
const CLOCK: [&str; 7] = [
    "..#####..",
    ".#..#..#.",
    ".#..#..#.",
    ".#..##.#.",
    ".#.....#.",
    ".#.....#.",
    "..#####..",
];

const EFFORT: [[&str; 7]; 5] = [
    [
        ".........",
        ".........",
        ".........",
        ".........",
        "#........",
        "#........",
        "#.#.#.#.#",
    ],
    [
        ".........",
        ".........",
        ".........",
        "..#......",
        "#.#......",
        "#.#......",
        "#.#.#.#.#",
    ],
    [
        ".........",
        ".........",
        "....#....",
        "..#.#....",
        "#.#.#....",
        "#.#.#....",
        "#.#.#.#.#",
    ],
    [
        ".........",
        "......#..",
        "....#.#..",
        "..#.#.#..",
        "#.#.#.#..",
        "#.#.#.#..",
        "#.#.#.#.#",
    ],
    [
        "........#",
        "......#.#",
        "....#.#.#",
        "..#.#.#.#",
        "#.#.#.#.#",
        "#.#.#.#.#",
        "#.#.#.#.#",
    ],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    Model,
    Effort(Effort),
    Branch,
    Lines,
    Battery,
    Dollar,
    Clock,
}

impl Icon {
    pub(crate) fn rectangles(self, layout: &Layout, column: u16) -> Vec<Rectangle> {
        let dot = layout.icon_dot();
        let width = dot * f32::from(WIDE);
        let height = dot * f32::from(HIGH);
        let left = (layout.strip_x(column) + (CELLS * layout.advance() - width) / 2.0).round();
        let top = (layout.strip_text_top() + (layout.line_height() - height) / 2.0).round();

        self.rows()
            .iter()
            .zip(0..HIGH)
            .flat_map(|(row, row_index)| {
                runs(row).map(move |(start, length)| Rectangle {
                    left: left + f32::from(start) * dot,
                    top: top + f32::from(row_index) * dot,
                    width: f32::from(length) * dot,
                    height: dot,
                    ink: ICON_INK,
                })
            })
            .collect()
    }

    const fn rows(self) -> [&'static str; 7] {
        match self {
            Self::Model => SPARK,
            Self::Effort(effort) => EFFORT[level(effort)],
            Self::Branch => BRANCH,
            Self::Lines => LINES,
            Self::Battery => BATTERY,
            Self::Dollar => DOLLAR,
            Self::Clock => CLOCK,
        }
    }
}

const fn level(effort: Effort) -> usize {
    match effort {
        Effort::Low => 0,
        Effort::Medium => 1,
        Effort::High => 2,
        Effort::ExtraHigh => 3,
        Effort::Max => 4,
    }
}

fn runs(row: &str) -> impl Iterator<Item = (u8, u8)> + '_ {
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
#[path = "../../../../../tests/dot-desktop/canvas/mark/icon.rs"]
mod tests;
