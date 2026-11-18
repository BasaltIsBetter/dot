use std::time::Duration;

use super::*;

fn transcript(texts: &[&str], now: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    for text in texts {
        transcript.append((*text).to_owned(), now);
    }
    transcript
}

fn text_of(row: Option<Row<'_>>) -> Option<&str> {
    match row? {
        Row::Gap => Some("<gap>"),
        Row::Text { entry, row } => Some(&entry.text[entry.rows[row].clone()]),
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
    let long = "word ".repeat(30);

    let transcript = transcript(&[long.trim_end()], Instant::now());

    assert_eq!(transcript.total(), 3);
    let first = text_of(transcript.row(0)).unwrap();
    assert!(first.chars().count() <= TEXT_WIDTH + 1, "{first:?}");
}

#[test]
fn finds_the_right_entry_deep_in_a_long_history() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..10_000 {
        transcript.append(format!("entry {number}"), now);
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
    transcript.scroll_by(3.0, 2.0);
    let scrolled = transcript.window(2.0);

    transcript.append("e".to_owned(), now);

    assert_eq!((scrolled.first, scrolled.end), (2, 4));
    let window = transcript.window(2.0);
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
