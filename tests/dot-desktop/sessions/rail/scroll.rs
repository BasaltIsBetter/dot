use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::Layout;

const ONE_STEP: f32 = 110.0;

fn stage() -> Stage {
    Stage::of(&Layout::reflowed(PhysicalSize::new(2000, 1400), 2.0))
}

fn slots(count: usize) -> Slots {
    Slots::of(&stage(), count)
}

fn key(number: u64) -> SessionKey {
    SessionKey(number)
}

fn scrolled_to(offset: f32) -> RailScroll {
    RailScroll {
        offset,
        ..RailScroll::default()
    }
}

#[test]
fn centres_a_column_that_fits_and_never_scrolls_it() {
    let slots = slots(3);

    let placed = slots.place(500.0);

    assert!(slots.most().abs() < f32::EPSILON);
    assert_eq!(placed, slots.place(0.0));
    assert_eq!(placed.band, None);
    let above = placed.first - slots.top;
    let below = slots.top + slots.room - (placed.new_card[0] + placed.new_card[1]);
    assert!((above - below).abs() <= 1.0, "{above} {below}");
}

#[test]
fn scrolls_a_long_column_no_further_than_its_last_card() {
    let slots = slots(40);
    let most = slots.most();

    assert!((most - (slots.content - slots.view())).abs() < 0.01);
    assert!(most > 0.0);
    assert_eq!(slots.place(most + 1_000.0), slots.place(most));
    assert_eq!(slots.place(-50.0), slots.place(0.0));
    assert!((slots.place(0.0).first - slots.place(most).first - most).abs() <= 1.0);
}

#[test]
fn keeps_the_new_card_a_gap_under_the_band_of_cards() {
    let slots = slots(40);

    let placed = slots.place(0.0);

    let [_, high] = placed.band.unwrap();
    assert!((placed.new_card[0] - (high + stage().points(12.0))).abs() < 0.01);
    assert!((placed.new_card[1] - slots.new_card).abs() < f32::EPSILON);
    assert!((placed.first - slots.top.round()).abs() < f32::EPSILON);
}

#[test]
fn brings_a_card_into_sight_by_the_shortest_way() {
    let slots = slots(40);
    let below = to_points(20) * slots.pitch + slots.card - slots.view();

    assert!((slots.reveal(20, 0.0) - below).abs() < 0.01);
    assert!((slots.reveal(2, 900.0) - 2.0 * slots.pitch).abs() < 0.01);
    assert!(slots.reveal(3, 0.0).abs() < f32::EPSILON);
    assert!((slots.reveal(3, 2.0 * slots.pitch) - 2.0 * slots.pitch).abs() < 0.01);
}

#[test]
fn shows_the_top_of_a_card_taller_than_the_view() {
    let slots = Slots {
        card: 100.0,
        pitch: ONE_STEP,
        new_card: 10.0,
        top: 0.0,
        room: 70.0,
        content: 320.0,
    };

    assert!((slots.reveal(1, 0.0) - ONE_STEP).abs() < f32::EPSILON);
    assert!((slots.reveal(1, 2.0 * ONE_STEP) - ONE_STEP).abs() < f32::EPSILON);
}

#[test]
fn brings_the_session_shown_into_sight_only_when_it_changes() {
    let slots = slots(40);
    let mut scroll = RailScroll::default();
    scroll.follow((key(1), 0), &slots);
    assert!(scroll.offset().abs() < f32::EPSILON);

    scroll.scroll(Wheel::Lines(-3.0), &slots);
    scroll.follow((key(1), 0), &slots);
    assert!((scroll.offset() - 3.0 * slots.pitch).abs() < 0.01);

    scroll.follow((key(26), 25), &slots);
    let revealed = scroll.offset();
    assert!((revealed - slots.reveal(25, 3.0 * slots.pitch)).abs() < 0.01);
    scroll.scroll(Wheel::Lines(1.0), &slots);
    scroll.follow((key(26), 25), &slots);
    assert!((scroll.offset() - (revealed - slots.pitch)).abs() < 0.01);
}

#[test]
fn holds_a_shorter_column_within_its_new_end() {
    let mut scroll = scrolled_to(slots(40).most());
    scroll.shown = Some(key(1));

    scroll.follow((key(1), 0), &slots(20));

    assert!((scroll.offset() - slots(20).most()).abs() < 0.01);
    scroll.follow((key(1), 0), &slots(2));
    assert!(scroll.offset().abs() < f32::EPSILON);
}

#[test]
fn moves_a_card_per_wheel_line_and_follows_the_trackpad_pixel_for_pixel() {
    let slots = slots(40);
    let mut scroll = RailScroll::default();

    scroll.scroll(Wheel::Lines(-2.0), &slots);
    assert!((scroll.offset() - 2.0 * slots.pitch).abs() < 0.01);
    scroll.scroll(Wheel::Pixels(-30.0), &slots);
    assert!((scroll.offset() - (2.0 * slots.pitch + 30.0)).abs() < 0.01);
    scroll.scroll(Wheel::Lines(1.0), &slots);
    assert!((scroll.offset() - (slots.pitch + 30.0)).abs() < 0.01);
}

#[test]
fn stops_at_both_ends_of_the_column() {
    let slots = slots(40);
    let mut scroll = RailScroll::default();

    scroll.scroll(Wheel::Lines(3.0), &slots);
    assert!(scroll.offset().abs() < f32::EPSILON);
    scroll.scroll(Wheel::Pixels(-100_000.0), &slots);
    assert!((scroll.offset() - slots.most()).abs() < 0.01);
}

#[test]
fn never_moves_a_column_that_fits() {
    let mut scroll = RailScroll::default();

    scroll.scroll(Wheel::Lines(-5.0), &slots(3));

    assert!(scroll.offset().abs() < f32::EPSILON);
}

#[test]
fn takes_the_wheel_only_over_the_column_and_above_the_status_bar() {
    let stage = stage();
    let [left, right] = stage.rail.unwrap();
    let middle = f32::midpoint(left, right);

    assert!(is_over_the_rail(&stage, [left, 10.0]));
    assert!(is_over_the_rail(&stage, [middle, stage.floor - 1.0]));
    assert!(!is_over_the_rail(&stage, [right, 10.0]));
    assert!(!is_over_the_rail(&stage, [left - 1.0, 10.0]));
    assert!(!is_over_the_rail(&stage, [middle, stage.floor]));
}

#[test]
fn gives_the_wheel_to_what_is_under_the_pointer_between_gestures() {
    let start = Instant::now();
    let mut scroll = RailScroll::default();

    assert!(scroll.takes_the_wheel(true, start));
    assert!(!scroll.takes_the_wheel(false, start + GESTURE));
    assert!(scroll.takes_the_wheel(true, start + 2 * GESTURE));
}

#[test]
fn keeps_the_wheel_where_a_gesture_started_while_it_goes_on() {
    let start = Instant::now();
    let step = Duration::from_millis(200);
    let mut on_the_rail = RailScroll::default();
    let mut on_the_transcript = RailScroll::default();

    assert!(on_the_rail.takes_the_wheel(true, start));
    assert!(on_the_rail.takes_the_wheel(false, start + step));
    assert!(on_the_rail.takes_the_wheel(false, start + 2 * step));
    assert!(!on_the_transcript.takes_the_wheel(false, start));
    assert!(!on_the_transcript.takes_the_wheel(true, start + step));
}
