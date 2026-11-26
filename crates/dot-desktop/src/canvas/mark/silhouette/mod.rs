mod outline;

use dot_transcript::{index, opacity};

use crate::canvas::mark::shape::{Paint, Rectangle};

const SAMPLES_DOWN: u16 = 8;
const FAINTEST: u8 = 16;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Silhouette {
    scale: f32,
    size: [f32; 2],
    runs: Vec<Run>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    left: u16,
    top: u16,
    width: u16,
    cover: u8,
}

impl Silhouette {
    pub(crate) fn at(scale: f32) -> Self {
        let exact = f64::from(scale);
        let [columns, rows] = pixels_at(exact);
        let mut runs = Vec::new();
        let mut covered = vec![0.0; usize::from(columns)];
        for row in 0..rows {
            covered.fill(0.0);
            for sample in 0..SAMPLES_DOWN {
                let down = (f64::from(row) + sample_middle(sample)) / exact;
                if let Some([left, right]) = outline::span_at(down) {
                    cover(&mut covered, [left * exact, right * exact]);
                }
            }
            push_runs(&mut runs, row, &covered);
        }
        Self {
            scale,
            size: [f32::from(columns), f32::from(rows)],
            runs,
        }
    }

    pub(crate) fn size_at(scale: f32) -> [f32; 2] {
        pixels_at(f64::from(scale)).map(f32::from)
    }

    pub(crate) const fn is_at(&self, scale: f32) -> bool {
        self.scale.to_bits() == scale.to_bits()
    }

    pub(crate) const fn size(&self) -> [f32; 2] {
        self.size
    }

    pub(crate) fn drawn(
        &self,
        [left, top]: [f32; 2],
        paint: Paint,
    ) -> impl Iterator<Item = Rectangle> + '_ {
        let [left, top] = [left.round(), top.round()];
        self.runs.iter().map(move |run| Rectangle {
            left: left + f32::from(run.left),
            top: top + f32::from(run.top),
            width: f32::from(run.width),
            height: 1.0,
            color: paint.dimmed(run.cover),
        })
    }
}

fn sample_middle(sample: u16) -> f64 {
    (f64::from(sample) + 0.5) / f64::from(SAMPLES_DOWN)
}

fn cover(covered: &mut [f64], [left, right]: [f64; 2]) {
    for (column, share) in (0_u16..).zip(covered.iter_mut()) {
        let column = f64::from(column);
        let overlap = right.min(column + 1.0) - left.max(column);
        if overlap > 0.0 {
            *share += overlap;
        }
    }
}

fn push_runs(runs: &mut Vec<Run>, row: u16, covered: &[f64]) {
    for (column, share) in (0_u16..).zip(covered) {
        let cover = opacity(share / f64::from(SAMPLES_DOWN));
        if cover < FAINTEST {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.top == row && run.cover == cover && run.left + run.width == column => {
                run.width += 1;
            }
            _ => runs.push(Run {
                left: column,
                top: row,
                width: 1,
                cover,
            }),
        }
    }
}

fn pixels_at(scale: f64) -> [u16; 2] {
    [outline::WIDTH, outline::HEIGHT].map(|points| pixels(points * scale))
}

fn pixels(exact: f64) -> u16 {
    u16::try_from(index(exact.ceil())).unwrap_or(u16::MAX)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/canvas/mark/silhouette/mod.rs"]
mod tests;
