use std::time::Instant;

use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::Layout;
use crate::conversation::transcript::entry::Kind;
use crate::conversation::transcript::ghost::Fate;
use crate::conversation::transcript::history::Row;
use crate::conversation::transcript::history::tests::{
    VISIBLE, after, long_entries, text_of, texts, transcript,
};

fn narrowest() -> TextWidth {
    TextWidth::of(&Layout::reflowed(PhysicalSize::new(0, 0), 1.0))
}

fn top_of(transcript: &Transcript) -> (Option<(String, usize)>, f64) {
    let window = transcript.window(VISIBLE, Instant::now());
    let row = match transcript.row(window.first) {
        Some(Row::Text { entry, row, .. }) => Some((entry.text.clone(), entry.row(row).start)),
        Some(Row::Gap) => Some(("<gap>".to_owned(), 0)),
        None => None,
    };
    (row, window.offset)
}

#[test]
fn rewraps_every_entry_and_the_ones_that_follow_to_the_new_width() {
    let now = Instant::now();
    let mut transcript = transcript(&["word word word word word word word"], now);

    transcript.rewrap(narrowest(), VISIBLE);
    transcript.append("one two three four five six seven", now);

    assert_eq!(narrowest().columns(), 28);
    assert_eq!(
        texts(&transcript),
        vec![
            Some("word word word word word "),
            Some("word word"),
            Some("<gap>"),
            Some("one two three four five six "),
            Some("seven"),
        ]
    );
}

#[test]
fn keeps_the_row_at_the_top_of_a_scrolled_view_when_the_width_changes() {
    let mut transcript = long_entries(40);
    transcript.scroll_by(31.0, VISIBLE, Instant::now());
    let (wide, offset) = top_of(&transcript);

    transcript.rewrap(narrowest(), VISIBLE);
    let narrow = top_of(&transcript);
    transcript.rewrap(TextWidth::default(), VISIBLE);

    let (text, start) = wide.clone().unwrap();
    assert!(text.starts_with("entry 21 "), "{text}");
    assert_eq!(start, 0);
    assert_eq!(narrow, (wide.clone(), offset));
    assert_eq!(top_of(&transcript), (wide, offset));
}

#[test]
fn keeps_a_row_inside_an_entry_at_the_top_when_it_widens() {
    let mut transcript = long_entries(40);
    transcript.rewrap(narrowest(), VISIBLE);
    transcript.scroll_by(28.0, VISIBLE, Instant::now());
    let (narrow, _) = top_of(&transcript);

    transcript.rewrap(TextWidth::default(), VISIBLE);

    let (text, start) = narrow.unwrap();
    assert!(start > 0, "{text}: {start}");
    assert_eq!(top_of(&transcript).0, Some((text, 0)));
}

#[test]
fn keeps_a_blank_row_at_the_top_when_the_width_changes() {
    let mut transcript = long_entries(40);
    transcript.scroll_by(30.0, VISIBLE, Instant::now());
    let (wide, _) = top_of(&transcript);

    transcript.rewrap(narrowest(), VISIBLE);
    let window = transcript.window(VISIBLE, Instant::now());

    assert_eq!(wide, Some(("<gap>".to_owned(), 0)));
    assert_eq!(top_of(&transcript).0, wide);
    assert_eq!(
        text_of(transcript.row(window.first + 1)),
        Some("entry 22 word word word word ")
    );
}

#[test]
fn keeps_part_of_a_row_scrolled_off_when_the_width_changes() {
    let mut transcript = long_entries(40);
    transcript.scroll_by(30.5, VISIBLE, Instant::now());
    let before = top_of(&transcript);

    transcript.rewrap(narrowest(), VISIBLE);

    assert!((before.1 + 0.5).abs() < f64::EPSILON, "{before:?}");
    assert_eq!(top_of(&transcript), before);
}

#[test]
fn keeps_following_the_end_when_the_width_changes() {
    let mut transcript = long_entries(40);

    transcript.rewrap(narrowest(), VISIBLE);

    let window = transcript.window(VISIBLE, Instant::now());
    assert_eq!(window.end, transcript.total());
    assert_eq!(
        text_of(transcript.row(window.end - 1)),
        Some("word word word")
    );
    transcript.append("latest", Instant::now());
    assert_eq!(
        text_of(transcript.row(transcript.window(VISIBLE, Instant::now()).end - 1)),
        Some("latest")
    );
}

#[test]
fn keeps_a_later_row_of_an_entry_at_the_top_when_it_narrows() {
    let texts: Vec<String> = (0..40)
        .map(|number| format!("entry {number:02} {}", "word ".repeat(22).trim_end()))
        .collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    let mut transcript = transcript(&texts, Instant::now());
    transcript.scroll_by(22.0, VISIBLE, Instant::now());
    let Some(Row::Text { entry, row, .. }) =
        transcript.row(transcript.window(VISIBLE, Instant::now()).first)
    else {
        panic!("the view should start on a row of text");
    };
    let (text, offset) = (entry.text.clone(), entry.row(row).start);
    assert_eq!(row, 1);

    transcript.rewrap(narrowest(), VISIBLE);

    let Some(Row::Text { entry, row, .. }) =
        transcript.row(transcript.window(VISIBLE, Instant::now()).first)
    else {
        panic!("the view should still start on a row of text");
    };
    assert_eq!(entry.text, text);
    assert!(row > 1, "{row}");
    assert!(
        entry.row(row).contains(&offset),
        "{offset} in {:?}",
        entry.row(row)
    );
}

#[test]
fn rewrapping_ends_the_travel_with_the_reply_at_the_top() {
    let start = Instant::now();
    let mut transcript = long_entries(20);
    transcript.append("hello", start);
    transcript.begin_reply(VISIBLE, start);
    transcript.hear(Kind::Answer, "reply ".repeat(30).trim_end(), VISIBLE, start);

    transcript.rewrap(narrowest(), VISIBLE);

    assert!(!transcript.is_traveling(after(start, 100)));
    let window = transcript.window(VISIBLE, after(start, 100));
    assert!(window.offset.abs() < f64::EPSILON);
    assert_eq!(
        text_of(transcript.row(window.first)),
        Some("reply reply reply reply ")
    );
}

#[test]
fn rewrapping_moves_the_pin_with_the_reply() {
    let start = Instant::now();
    let mut transcript = long_entries(20);
    transcript.append("hello", start);
    transcript.begin_reply(VISIBLE, start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);
    transcript.scroll_by(10.0, VISIBLE, after(start, 600));

    transcript.rewrap(narrowest(), VISIBLE);
    transcript.scroll_by(-1_000.0, VISIBLE, after(start, 600));

    let window = transcript.window(VISIBLE, after(start, 600));
    assert_eq!(text_of(transcript.row(window.first)), Some("Hi"));
    assert_eq!(window.first + 1, transcript.total());
}

#[test]
fn rewrapping_lets_the_words_already_reached_go_without_moving() {
    let start = Instant::now();
    let mut transcript = transcript(&["how are you"], start);
    transcript.begin_reply(VISIBLE, start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);

    transcript.rewrap(narrowest(), VISIBLE);

    let ghost = transcript.ghost().unwrap();
    assert_eq!(ghost.fate((0, 0), after(start, 100)), Fate::Gone);
    assert_eq!(ghost.fate((0, 8), after(start, 100)), Fate::Waiting);
}

#[test]
fn rewrapping_finds_the_last_word_of_the_question_at_the_new_width() {
    let start = Instant::now();
    let mut transcript = transcript(&["word word word word word word word"], start);
    transcript.begin_reply(VISIBLE, start);

    transcript.rewrap(narrowest(), VISIBLE);
    transcript.hear(Kind::Answer, "Hi\n", VISIBLE, start);

    assert_eq!(
        texts(&transcript),
        vec![Some("<gap>"), Some("Hi"), Some("")]
    );
}
