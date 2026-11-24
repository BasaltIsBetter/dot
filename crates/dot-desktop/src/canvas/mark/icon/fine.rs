use dot_transcript::{index, rows, to_f32};

use crate::canvas::layout::Layout;
use crate::canvas::mark::shape::{Paint, Rectangle};

const SIDE: f32 = 5.5;
const HEAD_SHARE: f32 = 0.6;
const ENTER_HEIGHT_SHARE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fine {
    Opens,
    Enter,
}

impl Fine {
    pub(crate) fn drawn(
        self,
        [left, top, width, height]: [f32; 4],
        layout: &Layout,
        color: Paint,
    ) -> Vec<Rectangle> {
        let side = layout.scaled(SIDE).max(Layout::HAIRLINE * 3.0);
        let tall = match self {
            Self::Opens => side,
            Self::Enter => (side * ENTER_HEIGHT_SHARE).round(),
        };
        let origin = [
            left + ((width - side) / 2.0).round(),
            top + ((height - tall) / 2.0).round(),
        ];
        let pen = Pen { origin, color };
        match self {
            Self::Opens => pen.opens(side),
            Self::Enter => pen.enter(side, tall),
        }
    }
}

struct Pen {
    origin: [f32; 2],
    color: Paint,
}

impl Pen {
    fn opens(&self, side: f32) -> Vec<Rectangle> {
        let head = (side * HEAD_SHARE).round();
        let mut drawn = self.diagonal([0.0, side - 1.0], side, -1.0);
        drawn.push(self.dot([side - head, 0.0], [head, 1.0]));
        drawn.push(self.dot([side - 1.0, 1.0], [1.0, head - 1.0]));
        drawn
    }

    fn enter(&self, side: f32, tall: f32) -> Vec<Rectangle> {
        let middle = (tall / 2.0).floor();
        let mut drawn = vec![
            self.dot([side - 1.0, 0.0], [1.0, middle]),
            self.dot([0.0, middle], [side, 1.0]),
        ];
        drawn.extend(self.diagonal([1.0, middle - 1.0], middle, -1.0));
        drawn.extend(self.diagonal([1.0, middle + 1.0], middle, 1.0));
        drawn
    }

    fn diagonal(&self, [x, y]: [f32; 2], steps: f32, down: f32) -> Vec<Rectangle> {
        (0..index(f64::from(steps)))
            .map(|step| {
                let step = to_f32(rows(step));
                self.dot([x + step, y + down * step], [1.0, 1.0])
            })
            .collect()
    }

    fn dot(&self, [x, y]: [f32; 2], [width, height]: [f32; 2]) -> Rectangle {
        let [left, top] = self.origin;
        Rectangle {
            left: left + x,
            top: top + y,
            width,
            height,
            color: self.color,
        }
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/canvas/mark/icon/fine.rs"]
mod tests;
