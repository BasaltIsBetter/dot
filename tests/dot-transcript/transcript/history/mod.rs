use std::time::Duration;

use super::*;

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
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Nice", now);
    transcript.end_reply(None, now);

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
fn adds_a_notice_below() {
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

#[test]
fn hands_over_the_rows_added_above_the_reply_once() {
    let start = Instant::now();
    let mut transcript = transcript(&["one", "how are you"], start);
    transcript.begin_reply(start);

    transcript.hear(Kind::Answer, "Fine thanks, and you?", after(start, 10));

    assert_eq!(
        transcript.take_insertions(),
        vec![Insertion { at: 2.0, rows: 2.0 }]
    );
    assert_eq!(transcript.take_insertions(), Vec::new());
}

#[test]
fn the_question_given_back_fades_in_once_its_last_words_left() {
    let start = Instant::now();
    let mut transcript = transcript(&["how"], start);
    transcript.begin_reply(start);

    transcript.hear(Kind::Answer, "Well, ", after(start, 10));

    let shown = |milliseconds| {
        transcript
            .returning(after(start, milliseconds))
            .map(|(question, shown)| (question, (shown * 1_000.0).round() / 1_000.0))
    };
    assert_eq!(shown(10), Some((0..1, 0.0)));
    assert_eq!(shown(309), Some((0..1, 0.0)));
    let halfway = shown(460).unwrap().1;
    assert!(halfway > 0.3 && halfway < 0.9, "{halfway}");
    assert_eq!(shown(610), None);
    assert!(transcript.is_animating(after(start, 460)));
}

#[test]
fn nothing_moves_long_after_the_last_arrival() {
    let start = Instant::now();
    let mut transcript = transcript(&["how"], start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Well, fine.", start);
    transcript.end_reply(None, start);

    assert!(transcript.is_animating(after(start, 500)));
    assert!(!transcript.is_animating(after(start, 2_000)));
}

#[test]
fn the_question_given_back_fades_in_from_its_last_word_while_the_reply_goes_on() {
    let start = Instant::now();
    let mut transcript = transcript(&["how"], start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Well, ", after(start, 10));

    transcript.hear(Kind::Answer, "it depends ", after(start, 200));
    transcript.hear(Kind::Answer, "on the day.", after(start, 400));

    let shown = transcript.returning(after(start, 460)).unwrap().1;
    assert!(shown > 0.3 && shown < 0.9, "{shown}");
}
