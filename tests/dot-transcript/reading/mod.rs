use std::time::{Duration, Instant};

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn plan(resting: usize, last: usize) -> Plan {
    Plan {
        resting,
        last,
        writing: None,
    }
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 0.001
}

fn settled_on(resting: usize, last: usize, now: Instant) -> Reader {
    let mut reader = Reader::default();
    reader.settle(plan(resting, last), now);
    reader
}

#[test]
fn starts_on_its_resting_row_without_gliding() {
    let start = Instant::now();

    let reader = settled_on(12, 20, start);

    assert!(close(reader.focus(start), 12.0));
    assert!(!reader.is_moving(start));
    assert!(reader.is_centred(start));
    assert!(!reader.is_reviewing());
}

#[test]
fn glides_to_a_new_resting_row_in_three_hundred_milliseconds() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);

    reader.settle(plan(14, 30), start);

    let halfway = reader.focus(after(start, 150));
    assert!(halfway > 10.0 && halfway < 14.0, "{halfway}");
    assert!(reader.is_moving(after(start, 150)));
    assert!(close(reader.focus(after(start, 300)), 14.0));
    assert!(!reader.is_moving(after(start, 300)));
}

#[test]
fn a_new_aim_glides_on_from_where_the_line_is_shown() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);
    reader.settle(plan(20, 30), start);
    let shown = reader.focus(after(start, 100));

    reader.settle(plan(12, 30), after(start, 100));

    assert!(close(reader.focus(after(start, 100)), shown));
    assert!(close(reader.focus(after(start, 400)), 12.0));
}

#[test]
fn scrolls_by_whole_rows_once_past_half_a_row() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);

    reader.scroll_by(0.4);
    reader.settle(plan(10, 30), start);
    assert!(!reader.is_reviewing());
    assert!(close(reader.focus(after(start, 300)), 10.0));

    reader.scroll_by(0.2);
    reader.settle(plan(10, 30), after(start, 300));
    assert!(reader.is_reviewing());
    assert!(close(reader.focus(after(start, 600)), 11.0));

    reader.scroll_by(-5.6);
    reader.settle(plan(10, 30), after(start, 600));
    assert!(close(reader.focus(after(start, 900)), 5.0));
}

#[test]
fn keeps_within_the_first_and_the_last_row() {
    let start = Instant::now();
    let mut reader = settled_on(10, 15, start);

    reader.scroll_by(-40.0);
    reader.settle(plan(10, 15), start);
    assert!(close(reader.focus(after(start, 300)), 0.0));

    reader.scroll_by(80.0);
    reader.settle(plan(10, 15), after(start, 300));
    assert!(close(reader.focus(after(start, 600)), 15.0));
}

#[test]
fn ignores_a_scroll_that_is_not_a_number() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);

    reader.scroll_by(f64::NAN);
    reader.scroll_by(f64::INFINITY);
    reader.settle(plan(10, 30), start);

    assert!(!reader.is_reviewing());
    assert!(close(reader.focus(start), 10.0));
}

#[test]
fn a_reader_looking_back_stays_where_it_reads_while_the_conversation_moves() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);
    reader.scroll_by(-6.0);
    reader.settle(plan(20, 40), start);
    let read = reader.focus(after(start, 300));

    reader.settle(plan(25, 45), after(start, 300));
    reader.settle(plan(31, 52), after(start, 400));

    assert!(close(read, 14.0));
    assert!(close(reader.focus(after(start, 400)), read));
    assert!(close(reader.focus(after(start, 900)), read));
}

#[test]
fn comes_back_to_the_resting_row() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);
    reader.scroll_by(-6.0);
    reader.settle(plan(20, 40), start);

    reader.come_back();
    reader.settle(plan(20, 40), after(start, 300));

    assert!(!reader.is_reviewing());
    assert!(close(reader.focus(after(start, 600)), 20.0));
}

#[test]
fn rows_added_above_move_the_line_along_at_once() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);

    reader.shift(8.0, 3.0);
    reader.settle(plan(23, 43), start);

    assert!(close(reader.focus(start), 23.0));
    assert!(!reader.is_moving(start));
}

#[test]
fn rows_added_below_a_reader_looking_back_move_nothing() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);
    reader.scroll_by(-10.0);
    reader.settle(plan(20, 40), start);
    let read = reader.focus(after(start, 300));

    reader.shift(15.0, 3.0);
    reader.settle(plan(23, 43), after(start, 300));

    assert!(close(read, 10.0));
    assert!(close(reader.focus(after(start, 300)), 10.0));
    assert!(close(reader.focus(after(start, 700)), 10.0));
}

#[test]
fn rows_added_above_a_reader_looking_back_move_it_along() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);
    reader.scroll_by(-10.0);
    reader.settle(plan(20, 40), start);

    reader.shift(5.0, 3.0);
    reader.settle(plan(23, 43), after(start, 300));

    assert!(close(reader.focus(after(start, 300)), 13.0));
    assert!(close(reader.focus(after(start, 700)), 13.0));
}

#[test]
fn lands_at_once_after_the_flow_wrapped_again() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);

    reader.land_on(30.0);
    reader.settle(plan(26, 50), start);

    assert!(close(reader.focus(start), 26.0));
    assert!(!reader.is_moving(start));
}

#[test]
fn a_reader_looking_back_lands_on_the_row_it_read() {
    let start = Instant::now();
    let mut reader = settled_on(20, 40, start);
    reader.scroll_by(-8.0);
    reader.settle(plan(20, 40), start);

    reader.land_on(17.0);
    reader.settle(plan(26, 50), after(start, 300));

    assert!(reader.is_reviewing());
    assert!(close(reader.focus(after(start, 300)), 17.0));
}

#[test]
fn the_band_follows_the_row_claude_writes_on() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);

    reader.settle(
        Plan {
            resting: 10,
            last: 30,
            writing: Some(14),
        },
        start,
    );

    assert!(close(reader.band(start), 0.0));
    let halfway = reader.band(after(start, 150));
    assert!(halfway > 0.0 && halfway < 4.0, "{halfway}");
    assert!(close(reader.band(after(start, 300)), 4.0));
}

#[test]
fn the_band_comes_back_to_the_line_while_the_reader_looks_back() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);
    let writing = Plan {
        resting: 10,
        last: 30,
        writing: Some(14),
    };
    reader.settle(writing, start);

    reader.scroll_by(-3.0);
    reader.settle(writing, after(start, 300));

    assert!(close(reader.band(after(start, 600)), 0.0));
}

#[test]
fn is_not_centred_while_it_glides_or_looks_back() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);
    reader.settle(plan(20, 30), start);
    assert!(!reader.is_centred(after(start, 50)));
    assert!(reader.is_centred(after(start, 300)));

    reader.scroll_by(-2.0);
    reader.settle(plan(20, 30), after(start, 300));
    assert!(!reader.is_centred(after(start, 900)));
}

proptest::proptest! {
    #[test]
    fn never_reads_outside_the_rows(
        scrolls in proptest::collection::vec(-50.0_f64..50.0, 0..12),
        resting in 0_usize..60,
        extra in 0_usize..60,
    ) {
        let start = Instant::now();
        let last = resting + extra;
        let mut reader = settled_on(resting, last, start);
        let mut now = start;
        for rows in scrolls {
            reader.scroll_by(rows);
            reader.settle(plan(resting, last), now);
            now += Duration::from_millis(400);
            let focus = reader.focus(now);
            proptest::prop_assert!(focus >= -0.001, "{focus}");
            proptest::prop_assert!(focus <= f64::from(u32::try_from(last).unwrap()) + 0.001, "{focus}");
        }
    }
}

#[test]
fn aiming_again_at_the_same_row_lets_the_glide_end_on_time() {
    let start = Instant::now();
    let mut reader = settled_on(10, 30, start);
    reader.settle(plan(14, 30), start);

    reader.settle(plan(14, 30), after(start, 150));

    assert!(close(reader.focus(after(start, 300)), 14.0));
    assert!(!reader.is_moving(after(start, 300)));
}
