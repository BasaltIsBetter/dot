use std::time::{Duration, Instant};

use dot_transcript::{Ink, Line, Span};

use super::*;
use crate::app::tests::IdTranslator;

const LINE_HEIGHT: f32 = 16.0;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn turning_since(start: Instant) -> Suggestions {
    let mut suggestions = Suggestions::default();
    suggestions.turn(start);
    suggestions
}

fn text_at(suggestions: &Suggestions, now: Instant) -> String {
    suggestions.hint(now, &IdTranslator).text
}

fn line_of(text: &str) -> Line {
    Line::new(0, vec![Span::new(text, Ink::Receding)])
}

#[test]
fn shows_the_first_suggestion_still_until_it_turns() {
    let suggestions = Suggestions::default();
    let now = Instant::now();

    let hint = suggestions.hint(now, &IdTranslator);

    assert_eq!(
        hint,
        Hint {
            text: "suggestion-explain-project".to_owned(),
            leaving: None,
        }
    );
    assert!(!suggestions.is_rolling(now));
    assert_eq!(suggestions.next_change(now), None);
}

#[test]
fn keeps_each_suggestion_four_seconds_then_shows_the_next() {
    let start = Instant::now();
    let suggestions = turning_since(start);

    assert_eq!(
        text_at(&suggestions, after(start, 3_999)),
        "suggestion-explain-project"
    );
    assert_eq!(
        text_at(&suggestions, after(start, 4_000)),
        "suggestion-fix-bug"
    );
    assert_eq!(
        text_at(&suggestions, after(start, 8_000)),
        "suggestion-write-tests"
    );
}

#[test]
fn comes_back_to_the_first_after_the_last() {
    let start = Instant::now();
    let suggestions = turning_since(start);

    assert_eq!(
        text_at(&suggestions, after(start, 28_000)),
        "suggestion-run-tests"
    );
    assert_eq!(
        text_at(&suggestions, after(start, 32_000)),
        "suggestion-explain-project"
    );
}

#[test]
fn turning_twice_keeps_the_first_start() {
    let start = Instant::now();
    let mut suggestions = turning_since(start);

    suggestions.turn(after(start, 3_000));

    assert_eq!(
        text_at(&suggestions, after(start, 4_000)),
        "suggestion-fix-bug"
    );
}

#[test]
fn wakes_at_the_next_change() {
    let start = Instant::now();
    let suggestions = turning_since(start);

    assert_eq!(suggestions.next_change(start), Some(after(start, 4_000)));
    assert_eq!(
        suggestions.next_change(after(start, 5_000)),
        Some(after(start, 8_000))
    );
}

#[test]
fn rolls_the_previous_one_out_for_300_milliseconds() {
    let start = Instant::now();
    let suggestions = turning_since(start);

    let rolling = suggestions.hint(after(start, 4_150), &IdTranslator);
    let Some((leaving, progress)) = rolling.leaving.clone() else {
        panic!("no suggestion leaving: {rolling:?}");
    };

    assert_eq!(rolling.text, "suggestion-fix-bug");
    assert_eq!(leaving, "suggestion-explain-project");
    assert!(progress > 0.5 && progress < 1.0, "{progress}");
    assert!(suggestions.is_rolling(after(start, 4_150)));
    assert!(!suggestions.is_rolling(after(start, 4_300)));
    assert_eq!(
        suggestions.hint(after(start, 4_300), &IdTranslator).leaving,
        None
    );
    assert_eq!(
        suggestions.hint(after(start, 3_999), &IdTranslator).leaving,
        None
    );
}

#[test]
fn rolls_nothing_out_when_it_starts_turning() {
    let start = Instant::now();
    let suggestions = turning_since(start);

    assert_eq!(
        suggestions.hint(after(start, 100), &IdTranslator).leaving,
        None
    );
}

#[test]
fn holds_on_the_suggestion_shown_then_turns_on_from_it() {
    let start = Instant::now();
    let mut suggestions = turning_since(start);

    suggestions.hold(after(start, 9_000));

    assert_eq!(
        text_at(&suggestions, after(start, 60_000)),
        "suggestion-write-tests"
    );
    assert_eq!(suggestions.next_change(after(start, 60_000)), None);

    let resumed = after(start, 60_000);
    suggestions.turn(resumed);

    assert_eq!(
        suggestions.hint(after(resumed, 100), &IdTranslator).leaving,
        None
    );
    assert_eq!(
        text_at(&suggestions, after(resumed, 3_999)),
        "suggestion-write-tests"
    );
    assert_eq!(
        text_at(&suggestions, after(resumed, 4_000)),
        "suggestion-review-changes"
    );
}

#[test]
fn places_a_still_hint_on_its_row() {
    let hint = Hint {
        text: "hint".to_owned(),
        leaving: None,
    };

    let placed = hint.placed(100.0, LINE_HEIGHT, line_of);

    assert_eq!(placed.len(), 1);
    assert!((placed[0].top - 100.0).abs() < f32::EPSILON);
    assert_eq!(placed[0].line, line_of("hint"));
    assert_eq!(placed[0].clip, None);
}

#[test]
fn rolls_the_leaving_hint_up_and_the_new_one_in_from_below_fading() {
    let hint = Hint {
        text: "new".to_owned(),
        leaving: Some(("old".to_owned(), 0.25)),
    };

    let placed = hint.placed(100.0, LINE_HEIGHT, line_of);

    assert_eq!(placed.len(), 2);
    assert_eq!(placed[0].line, line_of("old").faded(0.75));
    assert!((placed[0].top - (100.0_f32 - 0.25 * 0.7 * LINE_HEIGHT).round()).abs() < f32::EPSILON);
    assert_eq!(placed[1].line, line_of("new").faded(0.25));
    assert!((placed[1].top - (100.0_f32 + 0.75 * 0.7 * LINE_HEIGHT).round()).abs() < f32::EPSILON);
    assert_eq!(placed[0].clip, None);
    assert_eq!(placed[1].clip, None);
}
