use super::*;
use crate::transcript::history::tests::{after, texts, transcript};
use crate::transcript::tool::{Aside, Glyph, Tool};

#[test]
fn veils_the_question_and_waits_where_it_was_typed() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "hello"], now);

    transcript.begin_reply(now);

    assert!(transcript.is_live());
    assert_eq!(texts(&transcript), vec![Some("one")]);
    assert_eq!(transcript.anchor(), Some(2));
    assert_eq!(transcript.writing_row(), Some(2));
    assert_eq!(transcript.end_row(), 3);
}

#[test]
fn a_reply_alone_in_the_transcript_starts_at_the_first_row() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.begin_reply(now);

    transcript.hear(Kind::Answer, "a\nb", now);

    assert_eq!(transcript.anchor(), Some(0));
    assert_eq!(texts(&transcript), vec![Some("a"), Some("b")]);
    assert_eq!(transcript.writing_row(), Some(1));
}

#[test]
fn the_first_question_ever_waits_on_the_first_row() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);

    transcript.begin_reply(now);

    assert_eq!(transcript.anchor(), Some(0));
    assert_eq!(transcript.total(), 0);
    assert_eq!(transcript.end_row(), 1);
}

#[test]
fn grows_the_reply_piece_by_piece_and_starts_a_new_entry_when_its_kind_changes() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(now);

    transcript.hear(Kind::Thinking, "Hm", now);
    transcript.hear(Kind::Thinking, ".", now);
    transcript.hear(Kind::Answer, "Hi\nthere", now);
    transcript.end_reply(None, now);

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
fn writes_on_the_last_row_of_the_reply() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "hello"], now);
    transcript.begin_reply(now);

    let waiting = transcript.anchor();
    transcript.hear(Kind::Answer, "a\nb\nc", now);

    assert_eq!(waiting, Some(2));
    assert_eq!(transcript.anchor(), Some(4));
    assert_eq!(transcript.writing_row(), Some(transcript.total() - 1));
    assert_eq!(transcript.writing_row(), Some(6));
}

#[test]
fn hears_nothing_once_the_reply_has_ended() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Hi", now);
    transcript.end_reply(None, now);

    transcript.hear(Kind::Answer, " late", now);

    assert!(!transcript.is_live());
    assert_eq!(transcript.anchor(), None);
    assert_eq!(transcript.writing_row(), None);
    assert_eq!(
        texts(&transcript),
        vec![Some("hello"), Some("<gap>"), Some("Hi")]
    );
}

#[test]
fn hears_nothing_outside_a_reply() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);

    transcript.hear(Kind::Answer, "Hi", now);
    transcript.end_reply(Some("never"), now);

    assert_eq!(texts(&transcript), vec![Some("hello")]);
}

#[test]
fn a_reply_that_wrote_nothing_gives_the_question_back_in_place() {
    let now = Instant::now();
    let mut silent = transcript(&["hello"], now);
    let mut failed = transcript(&["hello"], now);
    silent.begin_reply(now);
    failed.begin_reply(now);

    silent.end_reply(None, now);
    failed.end_reply(Some("reply-overloaded"), now);

    assert_eq!(texts(&silent), vec![Some("hello")]);
    assert!(silent.ghost().is_none());
    assert_eq!(silent.take_insertions(), Vec::new());
    assert_eq!(
        texts(&failed),
        vec![Some("hello"), Some("<gap>"), Some("reply-overloaded")]
    );
}

#[test]
fn keeps_finding_rows_after_a_reply_grew() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "two"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "a\nb\nc", now);
    transcript.end_reply(None, now);

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
fn a_new_reply_carries_on_below_the_last_one() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Hi", now);
    transcript.end_reply(None, now);

    transcript.append("again", now);
    transcript.begin_reply(now);
    let waiting = transcript.anchor();
    transcript.hear(Kind::Answer, "Yes", now);

    assert_eq!(waiting, Some(4));
    assert_eq!(transcript.anchor(), Some(6));
    assert_eq!(transcript.writing_row(), Some(6));
}

#[test]
fn rows_a_piece_adds_to_the_text_already_written_count_at_once() {
    let now = Instant::now();
    let mut transcript = transcript(&["one", "two"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "a", now);

    transcript.hear(Kind::Answer, "\nb\nc", now);

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
        ]
    );
}

#[test]
fn a_reply_that_only_summed_up_a_command_lets_the_rest_of_the_question_go() {
    let start = Instant::now();
    let mut transcript = transcript(&["please run the whole test suite now"], start);
    transcript.begin_reply(start);
    let command = Tool {
        glyph: Glyph::Command,
        label: "ls".to_owned(),
    };
    transcript.start_tool("t1".into(), command, start);
    transcript.end_tool(
        "t1",
        "✓".into(),
        Some(Aside {
            cut: "⌘ ls",
            kept: "  ✓ · 0,1 s",
        }),
        after(start, 100),
    );
    transcript.take_insertions();

    transcript.end_reply(None, after(start, 200));

    assert_eq!(transcript.take_insertions().len(), 1);
    assert!(transcript.returning(after(start, 200)).is_some());
}

#[test]
fn a_reply_to_a_note_goes_on_below_it_without_writing_over_it() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Hi there", now);
    transcript.end_reply(None, now);

    transcript.note("shorter please", now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Answer, "Sure", now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("hello"),
            Some("<gap>"),
            Some("Hi there"),
            Some("<gap>"),
            Some("shorter please"),
            Some("<gap>"),
            Some("Sure"),
        ]
    );
    assert_eq!(transcript.anchor(), Some(6));
    assert_eq!(transcript.exchanges(), 2);
    assert_eq!(transcript.exchange_at(2), Some(0));
    assert_eq!(transcript.exchange_at(4), Some(1));
    assert_eq!(transcript.active_from(), 0);
}

#[test]
fn gives_back_the_spare_room_of_the_reply_once_it_ends() {
    let start = Instant::now();
    let mut transcript = transcript(&["Bonjour"], start);
    transcript.begin_reply(start);
    for piece in 0..200 {
        transcript.hear(Kind::Answer, &format!("piece {piece} "), after(start, 10));
    }
    let answer = transcript.entries.last().unwrap();
    let spare_before = answer.text.capacity() - answer.text.len();

    transcript.end_reply(None, after(start, 20));

    let answer = transcript.entries.last().unwrap();
    assert!(spare_before > 0);
    assert_eq!(answer.text.capacity(), answer.text.len());
}
