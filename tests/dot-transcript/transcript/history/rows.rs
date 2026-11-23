use std::time::Instant;

use super::*;

use crate::transcript::Glyph;
use crate::transcript::entry::Kind;
use crate::transcript::ghost::Fate;
use crate::transcript::history::Row;
use crate::transcript::history::tests::{after, long_entries, text_of, texts, transcript};
use crate::transcript::tool::Tool;

const NARROWEST_TEXT: TextWidth = TextWidth::new(28);

fn narrowest() -> TextWidth {
    NARROWEST_TEXT
}

fn held_at(transcript: &Transcript, focus: f64) -> (Option<(String, usize)>, f64) {
    let row = match transcript.row(index(focus.floor())) {
        Some(Row::Text { entry, row, .. }) => Some((entry.text.clone(), entry.row(row).start)),
        Some(Row::Gap) => Some(("<gap>".to_owned(), 0)),
        None => None,
    };
    (row, focus - focus.floor())
}

#[test]
fn rewraps_every_entry_and_the_ones_that_follow_to_the_new_width() {
    let now = Instant::now();
    let mut transcript = transcript(&["word word word word word word word"], now);

    transcript.rewrap(narrowest(), 0.0, now);
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
fn keeps_the_row_read_when_the_width_changes_both_ways() {
    let now = Instant::now();
    let mut transcript = long_entries(40);
    let read = held_at(&transcript, 42.0);

    let narrow = transcript.rewrap(narrowest(), 42.0, now);
    let held_narrow = held_at(&transcript, narrow);
    let wide = transcript.rewrap(TextWidth::default(), narrow, now);

    let (text, start) = read.0.clone().unwrap();
    assert!(text.starts_with("entry 21 "), "{text}");
    assert_eq!(start, 0);
    assert_eq!(held_narrow, read);
    assert_eq!(held_at(&transcript, wide), read);
    assert!((wide - 42.0).abs() < f64::EPSILON);
}

#[test]
fn keeps_a_row_inside_an_entry_when_it_widens() {
    let now = Instant::now();
    let mut transcript = long_entries(40);
    let narrow = transcript.rewrap(narrowest(), 0.0, now);
    assert!(narrow.abs() < f64::EPSILON);
    let inside = 58.0;
    let (read, _) = held_at(&transcript, inside);

    let wide = transcript.rewrap(TextWidth::default(), inside, now);

    let (text, start) = read.unwrap();
    assert!(start > 0, "{text}: {start}");
    assert_eq!(held_at(&transcript, wide).0, Some((text, 0)));
}

#[test]
fn keeps_a_blank_row_read_when_the_width_changes() {
    let now = Instant::now();
    let mut transcript = long_entries(40);
    let read = held_at(&transcript, 41.0);

    let narrow = transcript.rewrap(narrowest(), 41.0, now);

    assert_eq!(read.0, Some(("<gap>".to_owned(), 0)));
    assert_eq!(held_at(&transcript, narrow), read);
    assert_eq!(
        text_of(transcript.row(index(narrow) + 1)),
        Some("entry 21 word word word word ")
    );
}

#[test]
fn keeps_the_part_of_a_row_read_when_the_width_changes() {
    let now = Instant::now();
    let mut transcript = long_entries(40);

    let narrow = transcript.rewrap(narrowest(), 42.5, now);

    assert!((narrow - narrow.floor() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn a_row_read_below_the_entries_keeps_its_distance_from_their_end() {
    let now = Instant::now();
    let mut transcript = long_entries(40);
    let total = rows(transcript.total());

    let narrow = transcript.rewrap(narrowest(), total + 1.0, now);

    assert!((narrow - (rows(transcript.total()) + 1.0)).abs() < f64::EPSILON);
}

#[test]
fn rewrapping_lets_the_words_already_reached_go_without_moving() {
    let start = Instant::now();
    let mut transcript = transcript(&["how are you"], start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Hi", start);

    transcript.rewrap(narrowest(), 0.0, start);

    let ghost = transcript.ghost().unwrap();
    assert_eq!(ghost.fate((0, 0), after(start, 100)), Fate::Gone);
    assert_eq!(ghost.fate((0, 8), after(start, 100)), Fate::Waiting);
}

#[test]
fn rewrapping_finds_the_last_word_of_the_question_at_the_new_width() {
    let start = Instant::now();
    let mut transcript = transcript(&["word word word word word word word"], start);
    transcript.begin_reply(start);

    transcript.rewrap(narrowest(), 0.0, start);
    transcript.hear(Kind::Answer, "Hi\n", start);

    assert_eq!(texts(&transcript), vec![Some("Hi"), Some("")]);
    assert!(!transcript.ghost().unwrap().is_complete());
}

#[test]
fn rewrapping_past_the_last_word_gives_the_question_back() {
    let start = Instant::now();
    let question = "word ".repeat(24);
    let question = question.trim_end();
    let mut transcript = transcript(&[question], start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "a\nb", start);
    let before = transcript.ghost().unwrap().is_complete();

    transcript.rewrap(TextWidth::new(200), 0.0, after(start, 10));

    assert!(!before);
    assert!(transcript.ghost().unwrap().is_complete());
    assert_eq!(text_of(transcript.row(0)), Some(question));
}

#[test]
fn a_tool_without_a_sum_takes_no_row_and_no_gap() {
    let now = Instant::now();
    let mut transcript = transcript(&["look"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Reading.", now);
    let read = Tool {
        glyph: Glyph::Read,
        label: "a.rs".to_owned(),
    };

    transcript.start_tool("t1".into(), read, now);
    transcript.end_reply(None, now);

    assert_eq!(
        texts(&transcript),
        vec![Some("look"), Some("<gap>"), Some("Reading.")]
    );
    assert!(!transcript.takes_rows(3));
}

#[test]
fn reads_in_full_ink_from_the_last_prompt_on() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "two"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Hi", now);

    assert_eq!(transcript.active_from(), 2);
    assert_eq!(Transcript::default().active_from(), 0);
}

#[test]
fn without_a_prompt_nothing_is_read_in_full_ink() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.notice("conversation-restarted", now);

    assert_eq!(transcript.active_from(), transcript.total());
}

#[test]
fn ends_past_the_question_while_it_reaches_further_than_the_reply() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "a\nb\nc"], now);
    transcript.begin_reply(now);

    transcript.hear(Kind::Answer, "x", now);

    assert_eq!(transcript.total(), 3);
    assert_eq!(transcript.end_row(), 5);
}

#[test]
fn keeps_a_part_of_a_row_read_deep_in_an_entry_at_the_same_width() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "two\nlines\nhere"], now);

    let landed = transcript.rewrap(TextWidth::default(), 3.25, now);

    assert!((landed - 3.25).abs() < 1e-9, "{landed}");
}

#[test]
fn tells_the_exchange_of_a_row_from_the_nearest_text() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.notice("before anything", now);
    transcript.append("first", now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "one\ntwo", now);
    transcript.end_reply(None, now);
    transcript.append("second", now);

    let exchanges: Vec<Option<usize>> = (0..transcript.total())
        .map(|row| transcript.exchange_at(row))
        .collect();

    assert_eq!(
        exchanges,
        vec![
            None,
            None,
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            Some(1)
        ]
    );
    assert_eq!(transcript.exchanges(), 2);
    assert_eq!(transcript.exchange_at(transcript.total() + 1), Some(1));
    assert_eq!(transcript.exchange_at(transcript.total() + 3), None);
}
