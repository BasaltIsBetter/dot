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

fn pinned_at(row: f64) -> Scroll {
    let mut scroll = Scroll::default();
    scroll.pin(row);
    scroll
}

#[test]
fn brings_a_pinned_row_to_the_top_even_past_the_end() {
    let scroll = pinned_at(95.0);

    let window = scroll.window(TOTAL, VISIBLE);

    assert_eq!((window.first, window.end), (95, 100));
    assert!(close(window.offset, 0.0));
    assert_eq!(scroll.pinned(), Some(95.0));
}

#[test]
fn pins_a_row_at_the_top_of_short_content() {
    let window = pinned_at(2.0).window(3, VISIBLE);

    assert_eq!((window.first, window.end), (2, 3));
    assert!(close(window.offset, 0.0));
}

#[test]
fn shows_the_pinned_row_until_rows_below_pass_the_view_then_follows_the_end() {
    let scroll = pinned_at(95.0);

    assert_eq!(scroll.window(TOTAL + 5, VISIBLE).first, 95);
    assert_eq!(scroll.window(TOTAL + 6, VISIBLE).first, 96);
    assert_eq!(scroll.window(TOTAL + 30, VISIBLE).first, 120);
}

#[test]
fn tells_how_far_the_view_has_moved_past_its_pin() {
    let mut scroll = pinned_at(95.0);

    assert_eq!(Scroll::default().past_pin(TOTAL, VISIBLE), None);
    assert!(close(scroll.past_pin(TOTAL, VISIBLE).unwrap(), 0.0));
    assert!(close(scroll.past_pin(TOTAL + 30, VISIBLE).unwrap(), 25.0));
    scroll.by(20.0, TOTAL, VISIBLE);
    assert!(close(scroll.past_pin(TOTAL + 30, VISIBLE).unwrap(), -20.0));
}

#[test]
fn scrolling_back_down_stops_at_the_pinned_row_and_follows_the_end_again() {
    let mut scroll = pinned_at(95.0);
    scroll.by(20.0, TOTAL, VISIBLE);
    let above = scroll.window(TOTAL, VISIBLE);

    scroll.by(-50.0, TOTAL, VISIBLE);

    assert_eq!(above.first, 75);
    assert_eq!(scroll.first(), None);
    assert_eq!(scroll.window(TOTAL, VISIBLE).first, 95);
    assert_eq!(scroll.window(TOTAL + 30, VISIBLE).first, 120);
}

#[test]
fn follows_the_end_again_once_scrolled_to_it_past_the_pinned_row() {
    let mut scroll = pinned_at(50.0);

    scroll.by(-100.0, TOTAL, VISIBLE);

    assert_eq!(scroll.first(), None);
    let window = scroll.window(TOTAL + 30, VISIBLE);
    assert_eq!((window.first, window.end), (120, 130));
}

#[test]
fn keeps_a_pinned_view_on_its_row_when_refitted() {
    let mut scroll = pinned_at(95.0);

    scroll.refit(TOTAL, VISIBLE + 5.0);
    scroll.refit(TOTAL + 30, VISIBLE);

    assert_eq!(scroll.first(), None);
    assert_eq!(scroll.window(TOTAL, VISIBLE).first, 95);
}

#[test]
fn following_a_pinned_view_shows_the_pinned_row_while_the_rest_fits() {
    let mut scroll = pinned_at(95.0);

    scroll.follow();

    assert_eq!(scroll.window(TOTAL, VISIBLE).first, 95);
    assert_eq!(scroll.window(TOTAL + 30, VISIBLE).first, 120);
}

#[test]
fn moving_the_pin_moves_where_scrolling_down_stops() {
    let mut scroll = pinned_at(95.0);
    scroll.by(30.0, TOTAL, VISIBLE);

    scroll.move_pin(97.0);
    scroll.by(-100.0, TOTAL, VISIBLE);

    assert_eq!(scroll.pinned(), Some(97.0));
    assert_eq!(scroll.window(TOTAL, VISIBLE).first, 97);
}

#[test]
fn unpinning_the_view_on_its_pin_follows_the_end_again() {
    let mut scroll = pinned_at(95.0);

    scroll.unpin();

    assert_eq!(scroll, Scroll::default());
    assert_eq!(scroll.window(TOTAL, VISIBLE).first, 90);
}

#[test]
fn unpinning_keeps_a_view_scrolled_above_the_pin() {
    let mut scroll = pinned_at(95.0);
    scroll.by(30.0, TOTAL, VISIBLE);

    scroll.unpin();

    assert_eq!(scroll.pinned(), None);
    assert_eq!(scroll.first(), Some(65.0));
}

#[test]
fn unpinning_nothing_changes_nothing() {
    let mut scroll = Scroll::default();
    scroll.by(30.0, TOTAL, VISIBLE);
    let before = scroll;

    scroll.unpin();

    assert_eq!(scroll, before);
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
fn tells_the_top_row_short_content_sits_below() {
    assert!(close(Scroll::default().top(3, VISIBLE), -7.0));
    assert!(close(Scroll::default().top(TOTAL, VISIBLE), 90.0));
    assert!(close(pinned_at(2.0).top(3, VISIBLE), 2.0));
    assert!(close(pinned_at(0.0).top(3, VISIBLE), 0.0));
}

#[test]
fn a_view_of_one_row_pinned_on_the_only_row_follows_the_next_row_that_arrives() {
    let mut scroll = Scroll::default();
    scroll.pin(0.0);

    scroll.by(-1_000.0, 1, 1.0);

    assert_eq!(scroll.window(2, 1.0).first, 1);
    assert_eq!(scroll.window(1, 1.0).first, 0);
}

proptest! {
    #[test]
    fn the_window_never_leaves_the_rows(
        total in 0_usize..500,
        visible in 0.0_f64..40.0,
        pinned in proptest::option::of(0.0_f64..1.0),
        moves in prop::collection::vec(-60.0_f64..60.0, 0..8),
    ) {
        let mut scroll = Scroll::default();
        if let Some(share) = pinned {
            scroll.pin((share * rows(total)).floor());
        }
        for rows_up in moves {
            scroll.by(rows_up, total, visible);
        }

        let window = scroll.window(total, visible);

        prop_assert!(window.first <= window.end);
        prop_assert!(window.end <= total);
        prop_assert!(rows(window.end - window.first) <= visible.ceil() + 1.0);
        prop_assert!(window.offset > -1.0);
    }

    #[test]
    fn a_pinned_view_brought_back_down_shows_its_row_until_the_end_passes_it(
        pinned in 0_usize..300,
        below in 0_usize..40,
        visible in 1.0_f64..40.0,
        moves in prop::collection::vec(-60.0_f64..60.0, 0..8),
        arriving in 0_usize..500,
    ) {
        let total = pinned + 1 + below;
        prop_assume!(rows(pinned) >= rows(total) - visible);
        let mut scroll = Scroll::default();
        scroll.pin(rows(pinned));
        for rows_up in moves {
            scroll.by(rows_up, total, visible);
        }

        scroll.by(-1_000.0, total, visible);

        let end_top = rows(total + arriving) - visible;
        let expected = index(end_top.max(rows(pinned)).floor());
        prop_assert_eq!(scroll.window(total + arriving, visible).first, expected);
    }
}
