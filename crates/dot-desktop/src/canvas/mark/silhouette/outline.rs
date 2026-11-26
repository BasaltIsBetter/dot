pub(super) const WIDTH: f64 = 9.0;
pub(super) const HEIGHT: f64 = 11.0;
const MIDDLE: f64 = WIDTH / 2.0;
const HEAD_CENTRE: f64 = 2.6;
const HEAD_RADIUS: f64 = 2.3;
const SHOULDERS: [[f64; 2]; 4] = [[0.5, 11.0], [0.5, 7.4], [2.3, 5.9], [MIDDLE, 5.9]];
const SOLVER_STEPS: u32 = 24;
const ACROSS: usize = 0;
const DOWN: usize = 1;

pub(super) fn span_at(down: f64) -> Option<[f64; 2]> {
    let half = head_half_width(down).or_else(|| shoulders_half_width(down))?;
    Some([MIDDLE - half, MIDDLE + half])
}

fn head_half_width(down: f64) -> Option<f64> {
    let from_centre = down - HEAD_CENTRE;
    let squared = HEAD_RADIUS.mul_add(HEAD_RADIUS, -from_centre * from_centre);
    (squared > 0.0).then(|| squared.sqrt())
}

fn shoulders_half_width(down: f64) -> Option<f64> {
    let [bottom, .., top] = SHOULDERS;
    if !(top[DOWN]..=bottom[DOWN]).contains(&down) {
        return None;
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..SOLVER_STEPS {
        let middle = f64::midpoint(low, high);
        if curve(DOWN, middle) > down {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(MIDDLE - curve(ACROSS, f64::midpoint(low, high)))
}

fn curve(axis: usize, along: f64) -> f64 {
    let before = 1.0 - along;
    let [start, first, second, end] = SHOULDERS.map(|point| point[axis]);
    before * before * before * start
        + 3.0 * before * before * along * first
        + 3.0 * before * along * along * second
        + along * along * along * end
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/canvas/mark/silhouette/outline.rs"]
mod tests;
