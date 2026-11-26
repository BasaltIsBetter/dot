use dot_transcript::opacity;

use crate::canvas::{Layout, Paint, Rectangle, corners};

pub(super) const CONTOUR: f64 = 0.35;
const LINE: f32 = Layout::HAIRLINE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RowSpan {
    pub(super) left: f32,
    pub(super) right: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Marks {
    pub(super) color: Paint,
    pub(super) wash: f64,
    pub(super) arm: f32,
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct Outline {
    pub(super) spans: Vec<RowSpan>,
    pub(super) opens: bool,
    pub(super) closes: bool,
}

impl RowSpan {
    pub(super) fn filled(self, color: Paint) -> Rectangle {
        Rectangle {
            left: self.left,
            top: self.top,
            width: self.right - self.left,
            height: self.bottom - self.top,
            color,
        }
    }

    fn bounds(self) -> [f32; 4] {
        [
            self.left,
            self.top,
            self.right - self.left,
            self.bottom - self.top,
        ]
    }

    fn ringed(self) -> [f32; 4] {
        [
            self.left - LINE,
            self.top - LINE,
            self.right - self.left + 2.0 * LINE,
            self.bottom - self.top + 2.0 * LINE,
        ]
    }
}

impl Outline {
    pub(super) fn wash(&self, color: Paint) -> impl Iterator<Item = Rectangle> + '_ {
        self.spans.iter().map(move |span| span.filled(color))
    }

    pub(super) fn frame(&self, marks: Marks, into: &mut Vec<Rectangle>) {
        if self.spans.len() > 1 {
            let outside = marks.color.dimmed(opacity(CONTOUR));
            let over_the_wash = CONTOUR + (1.0 - CONTOUR) * marks.wash.clamp(0.0, 1.0);
            self.ring_outside(outside, into);
            self.ring_inside(marks.color.dimmed(opacity(over_the_wash)), into);
        }
        let Marks { color, arm, .. } = marks;
        if let Some(first) = self.spans.first().filter(|_| self.opens) {
            for bounds in [first.ringed(), first.bounds()] {
                let [left, left_down, right, right_down, ..] = corners(bounds, arm, color);
                into.extend([left, left_down, right, right_down]);
            }
        }
        if let Some(last) = self.spans.last().filter(|_| self.closes) {
            for bounds in [last.ringed(), last.bounds()] {
                let [.., left, left_up, right, right_up] = corners(bounds, arm, color);
                into.extend([left, left_up, right, right_up]);
            }
        }
    }

    fn ring_outside(&self, color: Paint, into: &mut Vec<Rectangle>) {
        let (Some(first), Some(last)) = (self.spans.first(), self.spans.last()) else {
            return;
        };
        for span in &self.spans {
            into.push(upright(span.left - LINE, span, color));
            into.push(upright(span.right, span, color));
        }
        into.push(across(widened(first), first.top - LINE, color));
        into.push(across(widened(last), last.bottom, color));
        for pair in self.spans.windows(2) {
            let [above, below] = [pair[0], pair[1]];
            let under_above = uncovered(widened(&below), reach(&above));
            let over_below = uncovered(widened(&above), reach(&below));
            into.extend(under_above.map(|piece| across(piece, above.bottom - LINE, color)));
            into.extend(over_below.map(|piece| across(piece, below.top, color)));
        }
    }

    fn ring_inside(&self, color: Paint, into: &mut Vec<Rectangle>) {
        let (Some(first), Some(last)) = (self.spans.first(), self.spans.last()) else {
            return;
        };
        for span in &self.spans {
            into.push(upright(span.left, span, color));
            into.push(upright(span.right - LINE, span, color));
        }
        into.push(across(reach(first), first.top, color));
        into.push(across(reach(last), last.bottom - LINE, color));
        for pair in self.spans.windows(2) {
            let [above, below] = [pair[0], pair[1]];
            let bottom_of_above = uncovered(reach(&above), narrowed(&below));
            let top_of_below = uncovered(reach(&below), narrowed(&above));
            into.extend(bottom_of_above.map(|piece| across(piece, above.bottom - LINE, color)));
            into.extend(top_of_below.map(|piece| across(piece, below.top, color)));
        }
    }
}

fn reach(span: &RowSpan) -> [f32; 2] {
    [span.left, span.right]
}

fn widened(span: &RowSpan) -> [f32; 2] {
    [span.left - LINE, span.right + LINE]
}

fn narrowed(span: &RowSpan) -> [f32; 2] {
    [span.left + LINE, span.right - LINE]
}

fn uncovered(
    [from, to]: [f32; 2],
    [hole_from, hole_to]: [f32; 2],
) -> impl Iterator<Item = [f32; 2]> {
    [[from, to.min(hole_from)], [from.max(hole_to), to]]
        .into_iter()
        .filter(|[from, to]| to > from)
}

fn upright(left: f32, span: &RowSpan, color: Paint) -> Rectangle {
    Rectangle {
        left,
        top: span.top,
        width: LINE,
        height: span.bottom - span.top,
        color,
    }
}

fn across([from, to]: [f32; 2], top: f32, color: Paint) -> Rectangle {
    Rectangle {
        left: from,
        top,
        width: to - from,
        height: LINE,
        color,
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/select/outline.rs"]
mod tests;
