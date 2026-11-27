use super::*;
use crate::ink::OPAQUE;
use crate::line::Line;
use crate::transcript::view::tests::{LINE, TRAVEL, row_top};

const BAND: Band = Band {
    top: 100.0,
    bottom: 100.0 + 4.0 * LINE,
};

fn place(first: usize, offset: f64) -> Place {
    Place {
        band: BAND,
        line_height: LINE,
        first,
        offset,
    }
}

#[test]
fn places_each_row_of_the_window_one_line_below_the_other() {
    let place = place(10, -0.25);

    assert_eq!(place.top_at(0), row_top(-0.25));
    assert_eq!(place.top_at(2), row_top(1.75));
    assert_eq!(place.top_of(12), Some(row_top(1.75)));
    assert_eq!(place.top_of(9), None);
}

#[test]
fn a_moving_line_travels_part_of_a_row_and_fades_on_its_way() {
    let line = Line::new(2, Vec::new());
    let top = row_top(1.0);

    let rising = place(0, 0.0).moving(top, Motion::Rising(0.25), line.clone());
    let leaving = place(0, 0.0).moving(top, Motion::Leaving(0.25), line.clone());
    let arrived = place(0, 0.0).moving(top, Motion::Rising(1.0), line);

    assert_eq!(rising.top, (top + 0.75 * TRAVEL * LINE).round());
    assert_eq!(leaving.top, (top - 0.25 * TRAVEL * LINE).round());
    assert_eq!(rising.line.alpha, 64);
    assert_eq!(leaving.line.alpha, 191);
    assert_eq!(arrived.top, top);
    assert_eq!(arrived.line.alpha, OPAQUE);
    assert_eq!(rising.clip, None);
    assert_eq!(leaving.clip, None);
}
