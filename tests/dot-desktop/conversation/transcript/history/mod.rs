use std::time::Duration;

use super::*;
use crate::conversation::scroll::rows;

pub(super) const VISIBLE: f64 = 6.0;

pub(super) fn transcript(texts: &[&str], now: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    for text in texts {
        transcript.append(text, now);
    }
    transcript
}

pub(super) fn text_of(row: Option<Row<'_>>) -> Option<&str> {
    match row? {
        Row::Gap => Some("<gap>"),
        Row::Text { entry, row, .. } => Some(&entry.text[entry.row(row)]),
    }
}

#[test]
fn starts_empty() {
    let transcript = Transcript::default();

    assert_eq!(transcript.total(), 0);
    assert_eq!(transcript.row(0), None);
    assert!(!transcript.is_fresh(Instant::now()));
}

#[test]
fn lays_entries_out_with_a_blank_row_between_them() {
    let transcript = transcript(&["first", "second\nline"], Instant::now());

    let rows: Vec<_> = (0..transcript.total())
        .map(|index| text_of(transcript.row(index)))
        .collect();

    assert_eq!(
        rows,
        vec![Some("first"), Some("<gap>"), Some("second"), Some("line")]
    );
    assert_eq!(transcript.row(4), None);
}

#[test]
fn wraps_an_entry_to_the_text_width() {
    let long = "word ".repeat(40);

    let transcript = transcript(&[long.trim_end()], Instant::now());

    assert_eq!(transcript.total(), 3);
    let first = text_of(transcript.row(0)).unwrap();
    assert!(
        first.chars().count() <= TextWidth::default().columns() + 1,
        "{first:?}"
    );
}

#[test]
fn finds_the_right_entry_deep_in_a_long_history() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..10_000 {
        transcript.append(&format!("entry {number}"), now);
    }

    assert_eq!(transcript.total(), 2 * 10_000 - 1);
    assert_eq!(text_of(transcript.row(0)), Some("entry 0"));
    assert_eq!(text_of(transcript.row(1)), Some("<gap>"));
    assert_eq!(text_of(transcript.row(12_345)), Some("<gap>"));
    assert_eq!(text_of(transcript.row(2 * 6_789)), Some("entry 6789"));
    assert_eq!(text_of(transcript.row(2 * 9_999)), Some("entry 9999"));
}

#[test]
fn follows_the_end_when_an_entry_arrives() {
    let now = Instant::now();
    let mut transcript = transcript(&["a", "b", "c", "d"], now);
    transcript.scroll_by(3.0, 2.0, now);
    let scrolled = transcript.window(2.0, now);

    transcript.append("e", now);

    assert_eq!((scrolled.first, scrolled.end), (2, 4));
    let window = transcript.window(2.0, now);
    assert_eq!((window.first, window.end), (7, 9));
}

#[test]
fn is_fresh_while_its_last_entry_settles() {
    let sent = Instant::now();
    let transcript = transcript(&["hello"], sent);

    assert!(transcript.is_fresh(sent));
    assert!(transcript.is_fresh(sent + Duration::from_millis(1_199)));
    assert!(!transcript.is_fresh(sent + Duration::from_millis(1_200)));
}

pub(super) fn texts(transcript: &Transcript) -> Vec<Option<&str>> {
    (0..transcript.total())
        .map(|index| text_of(transcript.row(index)))
        .collect()
}

#[test]
fn lists_the_images_of_a_prompt_right_under_it() {
    let now = Instant::now();
    let mut transcript = transcript(&["look"], now);

    transcript.attach("image  a.png  2 KB", now);
    transcript.attach("image  b.png  1.2 MB", now);
    transcript.begin_reply(VISIBLE, now);
    transcript.hear(Kind::Answer, "Nice", VISIBLE, now);
    transcript.end_reply(None, VISIBLE, now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("look"),
            Some("image  a.png  2 KB"),
            Some("image  b.png  1.2 MB"),
            Some("<gap>"),
            Some("Nice"),
        ]
    );
}

#[test]
fn adds_a_notice_below_and_follows_it() {
    let now = Instant::now();
    let mut transcript = transcript(&["look"], now);

    transcript.notice("image-unsupported", now);

    assert_eq!(
        texts(&transcript),
        vec![Some("look"), Some("<gap>"), Some("image-unsupported")]
    );
}

pub(super) fn long_entries(count: usize) -> Transcript {
    let texts: Vec<String> = (0..count)
        .map(|number| format!("entry {number:02} {}", "word ".repeat(7).trim_end()))
        .collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    transcript(&texts, Instant::now())
}

pub(super) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn top_at(transcript: &Transcript, now: Instant) -> f64 {
    let window = transcript.window(VISIBLE, now);
    rows(window.first) - window.offset
}

#[test]
fn a_wheel_during_the_travel_stops_the_view_where_it_is_shown() {
    let start = Instant::now();
    let mut transcript = long_entries(20);
    transcript.append("hello", start);
    transcript.begin_reply(VISIBLE, start);
    let shown = top_at(&transcript, after(start, 100));
    let was_traveling = transcript.is_traveling(after(start, 100));

    transcript.scroll_by(0.0, VISIBLE, after(start, 100));

    assert!(was_traveling);
    assert!(!transcript.is_traveling(after(start, 100)));
    assert!((top_at(&transcript, after(start, 100)) - shown).abs() < 1e-4);
    assert!((top_at(&transcript, after(start, 1_000)) - shown).abs() < 1e-4);
}

#[test]
fn a_wheel_during_the_travel_scrolls_on_from_where_the_view_is_shown() {
    let start = Instant::now();
    let mut transcript = long_entries(20);
    transcript.append("hello", start);
    transcript.begin_reply(VISIBLE, start);
    let shown = top_at(&transcript, after(start, 100));

    transcript.scroll_by(2.0, VISIBLE, after(start, 100));

    assert!((top_at(&transcript, after(start, 1_000)) - (shown - 2.0)).abs() < 1e-4);
}

#[test]
fn a_wheel_after_the_travel_scrolls_from_the_top_of_the_reply_to_the_question() {
    let start = Instant::now();
    let mut transcript = transcript(&["one", "two", "hello"], start);
    transcript.begin_reply(VISIBLE, start);
    transcript.hear(Kind::Answer, "Hi there", VISIBLE, start);

    transcript.scroll_by(2.0, VISIBLE, after(start, 600));

    let window = transcript.window(VISIBLE, after(start, 600));
    assert_eq!(text_of(transcript.row(window.first)), Some("hello"));
}

#[test]
fn settles_the_entries_that_have_cooled_when_another_arrives() {
    let start = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("first", start);
    transcript.append("second", start + std::time::Duration::from_millis(1_000));

    transcript.append("third", start + std::time::Duration::from_millis(1_500));

    assert_eq!(transcript.settled, 1);
    assert_eq!(transcript.entries[0].arrivals, []);
    assert_ne!(transcript.entries[1].arrivals, []);
    assert_ne!(transcript.entries[2].arrivals, []);
}
