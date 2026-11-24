use std::time::Duration;

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn focused_at(start: Instant) -> Blink {
    let mut blink = Blink::default();
    blink.pace(true, start);
    blink
}

#[test]
fn recedes_without_blinking_while_the_window_is_in_the_background() {
    let start = Instant::now();
    let mut blink = focused_at(start);

    blink.pace(false, after(start, 600));

    assert_eq!(blink.caret(after(start, 600)), Caret::Receded);
    assert_eq!(blink.next_change(after(start, 600)), None);
    assert!(Caret::Receded.is_shown());
}

#[test]
fn turns_on_and_off_every_half_period_from_when_it_started() {
    let start = Instant::now();
    let blink = focused_at(start);

    assert_eq!(blink.caret(start), Caret::Lit);
    assert_eq!(blink.caret(after(start, 529)), Caret::Lit);
    assert_eq!(blink.caret(after(start, 530)), Caret::Dark);
    assert_eq!(blink.caret(after(start, 1_059)), Caret::Dark);
    assert_eq!(blink.caret(after(start, 1_060)), Caret::Lit);
    assert!(!Caret::Dark.is_shown());
    assert!(Caret::Lit.is_shown());
}

#[test]
fn wakes_at_the_next_switch() {
    let start = Instant::now();
    let blink = focused_at(start);

    assert_eq!(blink.next_change(start), Some(after(start, 530)));
    assert_eq!(
        blink.next_change(after(start, 700)),
        Some(after(start, 1_060))
    );
}

#[test]
fn keeps_its_phase_while_the_window_stays_focused() {
    let start = Instant::now();
    let mut blink = focused_at(start);

    blink.pace(true, after(start, 600));

    assert_eq!(blink.caret(after(start, 600)), Caret::Dark);
}

#[test]
fn lights_up_again_on_a_key() {
    let start = Instant::now();
    let mut blink = focused_at(start);

    blink.restart(after(start, 600));

    assert_eq!(blink.caret(after(start, 600)), Caret::Lit);
    assert_eq!(
        blink.next_change(after(start, 600)),
        Some(after(start, 1_130))
    );
}

#[test]
fn a_key_in_the_background_does_not_start_it() {
    let start = Instant::now();
    let mut blink = Blink::default();

    blink.restart(start);

    assert_eq!(blink.caret(start), Caret::Receded);
}

#[test]
fn a_steady_caret_stays_lit_and_never_wakes_the_window() {
    let start = Instant::now();
    let mut blink = Blink::default();
    blink.hold(true);
    blink.pace(true, start);

    for half_periods in 0..4 {
        assert_eq!(blink.caret(start + HALF_PERIOD * half_periods), Caret::Lit);
    }
    assert_eq!(blink.next_change(start), None);
}

#[test]
fn a_steady_caret_still_recedes_when_the_window_is_in_the_background() {
    let mut blink = Blink::default();
    blink.hold(true);

    blink.pace(false, Instant::now());

    assert_eq!(blink.caret(Instant::now()), Caret::Receded);
}

#[test]
fn a_caret_blinks_again_once_no_longer_held() {
    let start = Instant::now();
    let mut blink = Blink::default();
    blink.hold(true);
    blink.pace(true, start);

    blink.hold(false);

    assert_eq!(blink.caret(start + HALF_PERIOD), Caret::Dark);
    assert!(blink.next_change(start).is_some());
}
