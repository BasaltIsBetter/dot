use proptest::prelude::*;

use super::*;

const TOTAL: usize = 100;
const VISIBLE: f64 = 10.0;

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

#[test]
fn follows_the_last_rows_by_default() {
    let window = Scroll::default().window(TOTAL, VISIBLE);

    assert_eq!((window.first, window.end), (90, 100));
    assert!(close(window.offset, 0.0));
}

#[test]
fn sets_short_content_at_the_bottom_of_the_view() {
    let window = Scroll::default().window(3, VISIBLE);

    assert_eq!((window.first, window.end), (0, 3));
    assert!(close(window.offset, 7.0));
}

#[test]
fn shows_nothing_when_there_is_nothing() {
    let window = Scroll::default().window(0, VISIBLE);

    assert_eq!((window.first, window.end), (0, 0));
}

#[test]
fn scrolling_up_shows_earlier_rows_cut_at_the_top() {
    let mut scroll = Scroll::default();

    scroll.by(2.5, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL, VISIBLE);
    assert_eq!((window.first, window.end), (87, 98));
    assert!(close(window.offset, -0.5));
}

#[test]
fn stops_at_the_first_row() {
    let mut scroll = Scroll::default();

    scroll.by(1_000.0, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL, VISIBLE);
    assert_eq!((window.first, window.end), (0, 10));
    assert!(close(window.offset, 0.0));
}

#[test]
fn scrolling_back_down_to_the_end_follows_again() {
    let mut scroll = Scroll::default();
    scroll.by(5.0, TOTAL, VISIBLE);

    scroll.by(-50.0, TOTAL, VISIBLE);

    assert_eq!(scroll, Scroll::default());
}

#[test]
fn keeps_the_view_still_when_rows_arrive_while_scrolled_up() {
    let mut scroll = Scroll::default();
    scroll.by(5.0, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL + 20, VISIBLE);

    assert_eq!((window.first, window.end), (85, 95));
}

#[test]
fn follows_rows_that_arrive_while_at_the_end() {
    let window = Scroll::default().window(TOTAL + 20, VISIBLE);

    assert_eq!((window.first, window.end), (110, 120));
}

#[test]
fn ignores_a_scroll_that_is_not_a_number() {
    let mut scroll = Scroll::default();
    scroll.by(3.0, TOTAL, VISIBLE);
    let before = scroll;

    scroll.by(f64::NAN, TOTAL, VISIBLE);
    scroll.by(f64::INFINITY, TOTAL, VISIBLE);

    assert_eq!(scroll, before);
}

#[test]
fn reveals_a_row_above_the_view_at_its_top() {
    let mut scroll = Scroll::default();

    scroll.reveal(40, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL, VISIBLE);
    assert_eq!((window.first, window.end), (40, 50));
}

#[test]
fn reveals_a_row_below_the_view_at_its_bottom() {
    let mut scroll = Scroll::default();
    scroll.by(50.0, TOTAL, VISIBLE);

    scroll.reveal(60, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL, VISIBLE);
    assert_eq!((window.first, window.end), (51, 61));
}

#[test]
fn brings_in_a_row_just_below_the_bottom_edge() {
    let mut scroll = Scroll::default();
    scroll.by(50.0, TOTAL, VISIBLE);

    scroll.reveal(50, TOTAL, VISIBLE);

    let window = scroll.window(TOTAL, VISIBLE);
    assert_eq!((window.first, window.end), (41, 51));
}

#[test]
fn leaves_a_visible_row_where_it_is() {
    let mut scroll = Scroll::default();
    scroll.by(20.0, TOTAL, VISIBLE);
    let before = scroll;

    for row in [70, 75, 79] {
        scroll.reveal(row, TOTAL, VISIBLE);

        assert_eq!(scroll, before, "row {row}");
    }
}

#[test]
fn revealing_the_last_row_follows_the_end_again() {
    let mut scroll = Scroll::default();
    scroll.by(20.0, TOTAL, VISIBLE);

    scroll.reveal(TOTAL - 1, TOTAL, VISIBLE);

    assert_eq!(scroll, Scroll::default());
}

#[test]
fn follows_again_when_a_taller_view_reaches_the_end() {
    let mut scroll = Scroll::default();
    scroll.by(3.0, TOTAL, VISIBLE);

    scroll.refit(TOTAL, VISIBLE + 5.0);

    assert_eq!(scroll, Scroll::default());
}

#[test]
fn keeps_its_place_when_refitted_short_of_the_end() {
    let mut scroll = Scroll::default();
    scroll.by(30.0, TOTAL, VISIBLE);
    let before = scroll;

    scroll.refit(TOTAL, VISIBLE + 5.0);
    scroll.refit(TOTAL, VISIBLE);

    assert_eq!(scroll, before);
}

#[test]
fn following_forgets_where_the_view_was() {
    let mut scroll = Scroll::default();
    scroll.by(20.0, TOTAL, VISIBLE);

    scroll.follow();

    assert_eq!(scroll, Scroll::default());
}

#[test]
fn a_view_with_no_height_shows_nothing() {
    for visible in [0.0, -3.0, f64::NAN] {
        let window = Scroll::default().window(TOTAL, visible);

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
        total in 0_usize..500,
        visible in 0.0_f64..40.0,
        moves in prop::collection::vec(-60.0_f64..60.0, 0..8),
    ) {
        let mut scroll = Scroll::default();
        for rows_up in moves {
            scroll.by(rows_up, total, visible);
        }

        let window = scroll.window(total, visible);

        prop_assert!(window.first <= window.end);
        prop_assert!(window.end <= total);
        prop_assert!(rows(window.end - window.first) <= visible.ceil() + 1.0);
        prop_assert!(window.offset > -1.0);
    }
}
