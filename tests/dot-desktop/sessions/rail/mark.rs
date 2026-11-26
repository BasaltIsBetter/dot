use std::time::Duration;

use super::*;
use crate::sessions::Foot;
use crate::sessions::rail::tests::{OWN, card};

#[test]
fn marks_each_state_with_its_glyph() {
    let at = Instant::now();
    let of = |here, state| glyph_of(&card("x", here, state, Foot::EMPTY));

    assert_eq!(of(true, State::Writing), ("◉", OWN));
    assert_eq!(of(true, State::Idle), ("◉", OWN));
    assert_eq!(of(false, State::Writing), ("●", OWN));
    assert_eq!(of(false, State::Paused), ("‖", Ink::Secondary));
    assert_eq!(of(false, State::Unread(at)), ("◆", Ink::Full));
    assert_eq!(of(false, State::Idle), ("○", Ink::Secondary));
}

#[test]
fn colours_the_corners_by_state_and_pointer() {
    let now = Instant::now();
    let colour =
        |here, state, hovered| corner_colour(&card("x", here, state, Foot::EMPTY), hovered, now);

    assert_eq!(colour(true, State::Writing, false), Paint::ink(OWN));
    assert_eq!(colour(true, State::Idle, true), Paint::ink(OWN));
    assert_eq!(colour(false, State::Writing, true), Paint::ink(OWN));
    assert_eq!(colour(false, State::Idle, true), Paint::ink(Ink::Full));
    assert_eq!(
        colour(false, State::Paused, false),
        Paint::ink(Ink::Receding)
    );
    assert_eq!(
        colour(false, State::Unread(now), false),
        blinking(Duration::ZERO)
    );
}

#[test]
fn blinks_from_receding_to_full_ink_and_back_in_one_point_four_seconds() {
    assert_eq!(blinking(Duration::ZERO).rgb(), Ink::Receding.rgb());
    assert_eq!(blinking(Duration::from_millis(700)).rgb(), Ink::Full.rgb());
    assert_eq!(
        blinking(Duration::from_millis(1_400)).rgb(),
        Ink::Receding.rgb()
    );
    let rising = blinking(Duration::from_millis(350)).rgb()[0];
    assert!(rising > 102 && rising < 255, "{rising}");
}

#[test]
fn draws_the_plus_of_the_new_card_as_two_strokes() {
    let [across, down] = plus([50.0, 40.0], 10.0, Paint::ink(Ink::Receding));

    assert_eq!((across.width, across.height), (10.0, 1.0));
    assert_eq!((down.width, down.height), (1.0, 10.0));
    assert_eq!(across.top, 40.0);
    assert_eq!(down.left, 50.0);
}
