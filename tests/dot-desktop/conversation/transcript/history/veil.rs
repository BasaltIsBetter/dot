use super::*;
use crate::conversation::transcript::ghost::Fate;
use crate::conversation::transcript::history::tests::{
    VISIBLE, after, long_entries, text_of, texts, transcript,
};

fn asked(texts: &[&str], now: Instant) -> Transcript {
    let mut transcript = transcript(texts, now);
    transcript.begin_reply(VISIBLE, now);
    transcript
}

fn top_row_text(transcript: &Transcript, now: Instant) -> Option<&str> {
    text_of(transcript.row(transcript.window(VISIBLE, now).first))
}

#[test]
fn hides_the_question_while_the_reply_waits() {
    let start = Instant::now();

    let transcript = asked(&["one", "hello"], start);

    assert_eq!(
        texts(&transcript),
        vec![Some("one"), Some("<gap>"), Some("")]
    );
    assert_eq!(transcript.ghost().map(Ghost::question), Some(1..2));
    assert_eq!(top_row_text(&transcript, after(start, 600)), Some(""));
}

#[test]
fn keeps_the_question_hidden_while_some_of_its_words_wait() {
    let start = Instant::now();
    let mut transcript = asked(&["one", "how are you"], start);

    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);

    assert_eq!(
        texts(&transcript),
        vec![Some("one"), Some("<gap>"), Some("Hi")]
    );
}

#[test]
fn brings_the_question_back_above_once_every_word_is_reached() {
    let start = Instant::now();
    let mut transcript = asked(&["one", "hello"], start);

    transcript.hear(Kind::Answer, "Hi there", VISIBLE, after(start, 700));

    assert_eq!(
        texts(&transcript),
        vec![
            Some("one"),
            Some("<gap>"),
            Some("hello"),
            Some("<gap>"),
            Some("Hi there"),
        ]
    );
    assert_eq!(
        top_row_text(&transcript, after(start, 700)),
        Some("Hi there")
    );
}

#[test]
fn brings_the_question_back_when_the_reply_ends_short_of_it() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);

    transcript.end_reply(None, VISIBLE, after(start, 700));

    assert_eq!(
        texts(&transcript),
        vec![Some("how are you"), Some("<gap>"), Some("Hi")]
    );
    assert!(transcript.ghost().is_some_and(Ghost::is_complete));
    assert_eq!(top_row_text(&transcript, after(start, 700)), Some("Hi"));
}

#[test]
fn brings_the_question_back_when_the_reply_moves_on_to_a_new_entry() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Thinking, "Hm", VISIBLE, start);

    transcript.hear(Kind::Answer, "Fine", VISIBLE, start);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("how are you"),
            Some("<gap>"),
            Some("Hm"),
            Some("<gap>"),
            Some("Fine"),
        ]
    );
}

#[test]
fn a_reply_that_wrote_nothing_shows_the_question_again_at_once() {
    let start = Instant::now();
    let mut transcript = asked(&["one", "hello"], start);

    transcript.end_reply(None, VISIBLE, start);

    assert_eq!(transcript.ghost(), None);
    assert_eq!(
        texts(&transcript),
        vec![Some("one"), Some("<gap>"), Some("hello")]
    );
}

#[test]
fn a_notice_in_place_of_the_reply_takes_the_place_of_the_question() {
    let start = Instant::now();
    let mut transcript = asked(&["hello"], start);

    transcript.end_reply(Some("reply-overloaded"), VISIBLE, start);

    assert_eq!(
        texts(&transcript),
        vec![Some("hello"), Some("<gap>"), Some("reply-overloaded")]
    );
    let ghost = transcript.ghost().unwrap();
    assert!(ghost.rolls(0));
    assert!(matches!(ghost.fate((0, 0), start), Fate::Leaving(_)));
}

#[test]
fn hides_the_images_of_the_question_with_it() {
    let start = Instant::now();
    let mut transcript = transcript(&["look"], start);
    transcript.attach("image  a.png  2 KB", start);

    transcript.begin_reply(VISIBLE, start);

    assert_eq!(texts(&transcript), vec![Some("<gap>"), Some("")]);
    assert_eq!(transcript.ghost().map(Ghost::question), Some(0..2));
}

#[test]
fn has_no_ghost_when_no_question_comes_before_the_reply() {
    let start = Instant::now();
    let mut alone = Transcript::default();
    let mut after_a_notice = transcript(&["hello"], start);
    after_a_notice.notice("image-unsupported", start);

    alone.begin_reply(VISIBLE, start);
    after_a_notice.begin_reply(VISIBLE, start);

    assert_eq!(alone.ghost(), None);
    assert_eq!(after_a_notice.ghost(), None);
}

#[test]
fn bringing_the_question_back_leaves_a_view_scrolled_above_it_in_place() {
    let start = Instant::now();
    let mut transcript = long_entries(20);
    transcript.append("hello", start);
    transcript.begin_reply(VISIBLE, start);
    transcript.scroll_by(10.0, VISIBLE, after(start, 600));
    let before = top_row_text(&transcript, after(start, 600)).map(str::to_owned);

    transcript.hear(Kind::Answer, "Hi", VISIBLE, after(start, 600));

    assert_eq!(
        top_row_text(&transcript, after(start, 600)).map(str::to_owned),
        before
    );
}

#[test]
fn the_reply_rolls_in_while_the_question_leaves() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);

    transcript.hear(Kind::Answer, "Hi", VISIBLE, after(start, 700));

    assert!(transcript.is_animating(after(start, 1_800)));
    assert!(!transcript.is_animating(after(start, 1_900)));
}

#[test]
fn the_question_leaving_at_the_end_keeps_the_window_moving() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Answer, "Hi", VISIBLE, start);

    transcript.end_reply(None, VISIBLE, after(start, 2_000));

    assert!(transcript.is_animating(after(start, 2_299)));
    assert!(!transcript.is_animating(after(start, 2_300)));
}
