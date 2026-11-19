use std::time::Duration;

use super::*;
use crate::conversation::scroll::rows;
use crate::conversation::transcript::history::tests::{VISIBLE, text_of, texts, transcript};

#[test]
fn waits_for_a_reply_on_one_empty_row_in_place_of_the_question() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);

    transcript.begin_reply(VISIBLE, now);

    assert!(transcript.is_live());
    assert_eq!(texts(&transcript), vec![Some("<gap>"), Some("")]);
    assert!(transcript.is_last_row(1));
    assert!(!transcript.is_last_row(0));
}

#[test]
fn grows_the_reply_piece_by_piece_and_starts_a_new_entry_when_its_kind_changes() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(VISIBLE, now);

    transcript.hear(Kind::Thinking, "Hm", VISIBLE, now);
    transcript.hear(Kind::Thinking, ".", VISIBLE, now);
    transcript.hear(Kind::Answer, "Hi\nthere", VISIBLE, now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("hello"),
            Some("<gap>"),
            Some("Hm."),
            Some("<gap>"),
            Some("Hi"),
            Some("there"),
        ]
    );
    assert!(transcript.is_fresh(now));
}

#[test]
fn hears_nothing_once_the_reply_has_ended() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(VISIBLE, now);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, now);
    transcript.end_reply(None, VISIBLE, now);

    transcript.hear(Kind::Answer, " late", VISIBLE, now);

    assert!(!transcript.is_live());
    assert_eq!(
        texts(&transcript),
        vec![Some("hello"), Some("<gap>"), Some("Hi")]
    );
}

#[test]
fn drops_the_waiting_row_of_a_reply_that_wrote_nothing() {
    let now = Instant::now();
    let mut silent = transcript(&["hello"], now);
    let mut failed = transcript(&["hello"], now);
    silent.begin_reply(VISIBLE, now);
    failed.begin_reply(VISIBLE, now);

    silent.end_reply(None, VISIBLE, now);
    failed.end_reply(Some("reply-overloaded"), VISIBLE, now);

    assert_eq!(texts(&silent), vec![Some("hello")]);
    assert_eq!(
        texts(&failed),
        vec![Some("hello"), Some("<gap>"), Some("reply-overloaded")]
    );
}

#[test]
fn keeps_finding_rows_after_a_reply_grew() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "two"], now);
    transcript.begin_reply(VISIBLE, now);
    transcript.hear(Kind::Answer, "a\nb\nc", VISIBLE, now);
    transcript.end_reply(None, VISIBLE, now);

    transcript.append("three", now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("one"),
            Some("<gap>"),
            Some("two"),
            Some("<gap>"),
            Some("a"),
            Some("b"),
            Some("c"),
            Some("<gap>"),
            Some("three"),
        ]
    );
}

#[test]
fn a_reply_alone_in_the_transcript_starts_at_the_first_row() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.begin_reply(VISIBLE, now);

    transcript.hear(Kind::Answer, "a\nb", VISIBLE, now);

    assert_eq!(texts(&transcript), vec![Some("a"), Some("b")]);
}

const REPLY_ROW: usize = 4;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn asked(start: Instant) -> Transcript {
    let mut transcript = transcript(&["one", "two", "hello"], start);
    transcript.begin_reply(VISIBLE, start);
    transcript
}

fn top_at(transcript: &Transcript, now: Instant) -> f64 {
    let window = transcript.window(VISIBLE, now);
    rows(window.first) - window.offset
}

fn shown(transcript: &Transcript, now: Instant) -> Vec<Option<&str>> {
    let window = transcript.window(VISIBLE, now);
    (window.first..window.end)
        .map(|index| text_of(transcript.row(index)))
        .collect()
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-4
}

#[test]
fn brings_the_reply_to_the_top_of_the_view() {
    let start = Instant::now();

    let transcript = asked(start);

    assert_eq!(text_of(transcript.row(REPLY_ROW)), Some(""));
    assert!(close(top_at(&transcript, after(start, 600)), 4.0));
    assert_eq!(shown(&transcript, after(start, 600)), vec![Some("")]);
}

#[test]
fn starts_with_the_waiting_row_at_the_bottom() {
    let start = Instant::now();

    let transcript = asked(start);

    assert!(close(top_at(&transcript, start), -1.0));
    assert_eq!(
        shown(&transcript, start).last(),
        Some(&text_of(transcript.row(REPLY_ROW)))
    );
}

#[test]
fn starts_with_every_row_of_the_question_above_the_bottom() {
    let start = Instant::now();
    let mut transcript = transcript(
        &[
            "one",
            "two",
            "hello
there
you",
        ],
        start,
    );

    transcript.begin_reply(VISIBLE, start);

    assert!(close(top_at(&transcript, start), 1.0));
    assert!(close(top_at(&transcript, after(start, 600)), 4.0));
}

#[test]
fn travels_up_along_the_pin_curve_for_six_hundred_milliseconds() {
    let start = Instant::now();

    let transcript = asked(start);

    let halfway = 4.0 - 5.0 * (1.0 - 0.946_079_461_5);
    assert!(close(top_at(&transcript, after(start, 300)), halfway));
    assert!(transcript.is_traveling(after(start, 599)));
    assert!(!transcript.is_traveling(after(start, 600)));
}

#[test]
fn keeps_the_reply_at_the_top_while_it_fits_in_the_view() {
    let start = Instant::now();
    let mut transcript = asked(start);

    transcript.hear(Kind::Answer, "1\n2\n3\n4\n5\n6", VISIBLE, after(start, 700));

    assert!(!transcript.is_traveling(after(start, 700)));
    assert!(close(top_at(&transcript, after(start, 700)), 6.0));
}

#[test]
fn glides_down_to_the_end_as_the_reply_grows_past_the_view() {
    let start = Instant::now();
    let mut transcript = asked(start);

    transcript.hear(
        Kind::Answer,
        "1\n2\n3\n4\n5\n6\n7\n8\n9",
        VISIBLE,
        after(start, 700),
    );

    assert!(close(top_at(&transcript, after(start, 700)), 6.0));
    let halfway = top_at(&transcript, after(start, 850));
    assert!(halfway > 6.0 && halfway < 9.0);
    assert!(transcript.is_traveling(after(start, 999)));
    assert!(!transcript.is_traveling(after(start, 1_000)));
    assert!(close(top_at(&transcript, after(start, 1_000)), 9.0));
    assert_eq!(shown(&transcript, after(start, 1_000))[0], Some("4"));
}

#[test]
fn glides_on_from_where_the_view_is_when_more_rows_arrive() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(
        Kind::Answer,
        "1\n2\n3\n4\n5\n6\n7",
        VISIBLE,
        after(start, 700),
    );
    let midway = top_at(&transcript, after(start, 800));

    transcript.hear(Kind::Answer, "\n8", VISIBLE, after(start, 800));

    assert!(close(top_at(&transcript, after(start, 800)), midway));
    assert!(close(top_at(&transcript, after(start, 1_100)), 8.0));
}

#[test]
fn a_piece_that_adds_no_row_leaves_the_glide_as_it_is() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(
        Kind::Answer,
        "1\n2\n3\n4\n5\n6\n7",
        VISIBLE,
        after(start, 700),
    );

    transcript.hear(Kind::Answer, "x", VISIBLE, after(start, 800));

    assert!(!transcript.is_traveling(after(start, 1_000)));
}

#[test]
fn never_glides_more_than_one_view() {
    let start = Instant::now();
    let mut transcript = asked(start);

    transcript.hear(
        Kind::Answer,
        &"row\n".repeat(30),
        VISIBLE,
        after(start, 700),
    );

    assert!(close(
        top_at(&transcript, after(start, 700)),
        31.0 - VISIBLE
    ));
    assert!(close(top_at(&transcript, after(start, 1_000)), 31.0));
}

#[test]
fn leaves_a_view_scrolled_above_the_reply_where_it_is() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.scroll_by(2.0, VISIBLE, after(start, 600));

    transcript.hear(
        Kind::Answer,
        &"row\n".repeat(30),
        VISIBLE,
        after(start, 700),
    );

    assert!(!transcript.is_traveling(after(start, 700)));
    assert!(close(top_at(&transcript, after(start, 700)), 2.0));
}

#[test]
fn glides_down_to_a_notice_that_passes_the_view() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(Kind::Answer, "1\n2\n3\n4\n5", VISIBLE, after(start, 700));

    transcript.end_reply(Some("reply-stopped"), VISIBLE, after(start, 700));

    assert!(transcript.is_traveling(after(start, 700)));
    assert_eq!(
        shown(&transcript, after(start, 1_000)).last(),
        Some(&Some("reply-stopped"))
    );
}

#[test]
fn follows_the_end_once_scrolled_back_down_to_the_reply() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);

    transcript.scroll_by(3.0, VISIBLE, after(start, 600));
    transcript.scroll_by(-10.0, VISIBLE, after(start, 600));
    transcript.hear(
        Kind::Answer,
        "\n1\n2\n3\n4\n5\n6\n7",
        VISIBLE,
        after(start, 600),
    );

    assert_eq!(
        shown(&transcript, after(start, 1_000)).last(),
        Some(&Some("7"))
    );
}

#[test]
fn follows_a_long_reply_again_once_scrolled_to_its_end() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(Kind::Answer, "1\n2\n3\n4\n5\n6\n7\n8\n9", VISIBLE, start);

    transcript.scroll_by(-100.0, VISIBLE, after(start, 600));
    transcript.hear(Kind::Answer, "\nlast", VISIBLE, after(start, 600));

    assert_eq!(
        shown(&transcript, after(start, 900)).last(),
        Some(&Some("last"))
    );
}

#[test]
fn travels_on_from_a_short_reply_still_at_the_top() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);
    transcript.end_reply(None, VISIBLE, start);
    let later = after(start, 1_000);

    transcript.append("again", later);
    transcript.begin_reply(VISIBLE, later);

    assert!(close(top_at(&transcript, later), 6.0));
    assert!(close(top_at(&transcript, after(later, 600)), 8.0));
    assert_eq!(shown(&transcript, after(later, 600)), vec![Some("")]);
}

#[test]
fn never_travels_more_than_one_view() {
    let start = Instant::now();
    let mut transcript = asked(start);
    transcript.end_reply(Some("reply-overloaded"), VISIBLE, start);

    transcript.append("again", start);
    transcript.begin_reply(VISIBLE, start);

    assert!(close(top_at(&transcript, start), 9.0 - VISIBLE));
    assert_eq!(shown(&transcript, start).last(), Some(&Some("")));
}

#[test]
fn a_view_shorter_than_a_row_jumps_to_the_reply() {
    let start = Instant::now();
    let mut transcript = transcript(&["hello"], start);

    transcript.begin_reply(0.5, start);

    assert!(!transcript.is_traveling(start));
    assert_eq!(transcript.window(0.5, start).first, 1);
}

#[test]
fn a_reply_that_failed_at_once_shows_its_notice_at_the_top() {
    let start = Instant::now();
    let mut transcript = asked(start);

    transcript.end_reply(Some("reply-overloaded"), VISIBLE, start);

    assert_eq!(
        shown(&transcript, after(start, 600)),
        vec![Some("reply-overloaded")]
    );
}

#[test]
fn a_reply_that_wrote_nothing_lets_the_view_follow_the_end_at_once() {
    let start = Instant::now();
    let mut transcript = asked(start);

    transcript.end_reply(None, VISIBLE, start);

    assert!(!transcript.is_traveling(start));
    let window = transcript.window(VISIBLE, start);
    assert_eq!((window.first, window.end), (0, 5));
    assert!(close(window.offset, 1.0));
    transcript.append("next", start);
    assert_eq!(shown(&transcript, start).last(), Some(&Some("next")));
}
