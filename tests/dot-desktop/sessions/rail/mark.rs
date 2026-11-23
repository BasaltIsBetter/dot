use std::time::Duration;

use super::*;
use crate::sessions::rail::tests::card;

#[test]
fn marks_each_state_with_its_glyph() {
    let at = Instant::now();
    let of = |here, state| glyph_of(&card("x", here, state, None));

    assert_eq!(of(true, State::Writing), ("◉", Ink::Full));
    assert_eq!(of(false, State::Writing), ("●", Ink::Live));
    assert_eq!(of(false, State::Paused), ("‖", Ink::Secondary));
    assert_eq!(of(false, State::Unread(at)), ("◆", Ink::Full));
    assert_eq!(of(false, State::Idle), ("○", Ink::Secondary));
}

#[test]
fn colours_the_corners_by_state_and_pointer() {
    let now = Instant::now();
    let colour = |here, state, hovered| corner_colour(&card("x", here, state, None), hovered, now);

    assert_eq!(colour(true, State::Writing, false), Rgba::ink(Ink::Full));
    assert_eq!(colour(false, State::Writing, true), Rgba::ink(Ink::Live));
    assert_eq!(colour(false, State::Idle, true), Rgba::ink(Ink::Full));
    assert_eq!(
        colour(false, State::Paused, false),
        Rgba::ink(Ink::Receding)
    );
    assert_eq!(
        colour(false, State::Unread(now), false),
        blinking(Duration::ZERO)
    );
}

#[test]
fn blinks_from_receding_to_full_ink_and_back_in_one_point_four_seconds() {
    assert_eq!(blinking(Duration::ZERO), Rgba::ink(Ink::Receding));
    assert_eq!(blinking(Duration::from_millis(700)), Rgba::ink(Ink::Full));
    assert_eq!(
        blinking(Duration::from_millis(1_400)),
        Rgba::ink(Ink::Receding)
    );
    let rising = blinking(Duration::from_millis(350)).rgb[0];
    assert!(rising > 102 && rising < 255, "{rising}");
}

#[test]
fn draws_the_plus_of_the_new_card_as_two_strokes() {
    let [across, down] = plus([50.0, 40.0], 10.0, Rgba::ink(Ink::Receding));

    assert_eq!((across.width, across.height), (10.0, 1.0));
    assert_eq!((down.width, down.height), (1.0, 10.0));
    assert_eq!(across.top, 40.0);
    assert_eq!(down.left, 50.0);
}

#[test]
fn draws_four_corners_of_two_strokes_each_inside_the_card() {
    let colour = Rgba::ink(Ink::Full);

    let strokes = corners([10.0, 20.0, 100.0, 50.0], 9.0, colour);

    let lefts: Vec<f32> = strokes.iter().map(|stroke| stroke.left).collect();
    let tops: Vec<f32> = strokes.iter().map(|stroke| stroke.top).collect();
    assert_eq!(lefts, [10.0, 10.0, 101.0, 109.0, 10.0, 10.0, 101.0, 109.0]);
    assert_eq!(tops, [20.0, 20.0, 20.0, 20.0, 69.0, 61.0, 69.0, 61.0]);
    let sizes: Vec<[f32; 2]> = strokes
        .iter()
        .map(|stroke| [stroke.width, stroke.height])
        .collect();
    assert_eq!(sizes, [[9.0, 1.0], [1.0, 9.0]].repeat(4));
}
