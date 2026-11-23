use proptest::prelude::*;

use super::*;

const TOTAL: usize = 100;
const VISIBLE: f64 = 10.0;

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

#[test]
fn places_a_window_wherever_its_top_is() {
    let below = Window::at(-3.0, TOTAL, VISIBLE);
    let between = Window::at(12.25, TOTAL, VISIBLE);
    let last = Window::at(95.0, TOTAL, VISIBLE);
    let beyond = Window::at(150.0, TOTAL, VISIBLE);

    assert_eq!((below.first, below.end), (0, 7));
    assert!(close(below.offset, 3.0));
    assert_eq!((between.first, between.end), (12, 23));
    assert!(close(between.offset, -0.25));
    assert_eq!((last.first, last.end), (95, 100));
    assert_eq!((beyond.first, beyond.end), (100, 100));
}

#[test]
fn a_view_with_no_height_shows_nothing() {
    for visible in [0.0, -3.0, f64::NAN] {
        let window = Window::at(40.0, TOTAL, visible);

        assert_eq!(window.first, window.end, "{visible}");
    }
}

#[test]
fn converts_between_rows_and_indices() {
    assert!(close(rows(42), 42.0));
    assert!(close(rows(usize::MAX), f64::from(u32::MAX)));
    assert_eq!(index(3.9), 3);
    assert_eq!(index(-2.0), 0);
    assert_eq!(index(f64::NAN), 0);
}

proptest! {
    #[test]
    fn the_window_never_leaves_the_rows(
        top in -60.0_f64..600.0,
        total in 0_usize..500,
        visible in 0.0_f64..40.0,
    ) {
        let window = Window::at(top, total, visible);

        prop_assert!(window.first <= window.end);
        prop_assert!(window.end <= total);
        prop_assert!(rows(window.end - window.first) <= visible.ceil() + 1.0);
        prop_assert!(window.offset > -1.0);
    }
}
