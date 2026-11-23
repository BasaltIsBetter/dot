use super::*;
use crate::transcript::ghost::Fate;
use crate::transcript::history::tests::{after, texts, transcript};

fn asked(texts: &[&str], now: Instant) -> Transcript {
    let mut transcript = transcript(texts, now);
    transcript.begin_reply(now);
    transcript
}

#[test]
fn hides_the_question_while_the_reply_waits() {
    let start = Instant::now();

    let transcript = asked(&["one", "hello"], start);

    assert_eq!(texts(&transcript), vec![Some("one")]);
    assert_eq!(transcript.ghost().map(Ghost::question), Some(1..2));
}

#[test]
fn keeps_the_question_hidden_while_some_of_its_words_wait() {
    let start = Instant::now();
    let mut transcript = asked(&["one", "how are you"], start);

    transcript.hear(Kind::Answer, "Hi", start);

    assert_eq!(
        texts(&transcript),
        vec![Some("one"), Some("<gap>"), Some("Hi")]
    );
    assert!(!transcript.ghost().unwrap().is_complete());
}

#[test]
fn brings_the_question_back_above_once_every_word_is_reached() {
    let start = Instant::now();
    let mut transcript = asked(&["one", "hello"], start);

    transcript.hear(Kind::Answer, "Hi there", after(start, 700));

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
    assert_eq!(transcript.anchor(), Some(4));
}

#[test]
fn brings_the_question_back_when_the_reply_ends_short_of_it() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Answer, "Hi", start);

    transcript.end_reply(None, after(start, 700));

    assert_eq!(
        texts(&transcript),
        vec![Some("how are you"), Some("<gap>"), Some("Hi")]
    );
    assert!(transcript.ghost().is_some_and(Ghost::is_complete));
}

#[test]
fn a_reply_in_several_entries_passes_the_question_as_a_whole() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Thinking, "Hm", start);
    let after_the_thinking = transcript.ghost().unwrap().is_complete();

    transcript.hear(Kind::Answer, "Fine", start);

    assert!(!after_the_thinking);
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

    transcript.end_reply(None, start);

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

    transcript.end_reply(Some("reply-overloaded"), start);

    assert_eq!(
        texts(&transcript),
        vec![Some("hello"), Some("<gap>"), Some("reply-overloaded")]
    );
    let ghost = transcript.ghost().unwrap();
    assert!(ghost.rolls(start));
    assert!(matches!(ghost.fate((0, 0), start), Fate::Leaving(_)));
}

#[test]
fn hides_the_images_of_the_question_with_it() {
    let start = Instant::now();
    let mut transcript = transcript(&["look"], start);
    transcript.attach("image  a.png  2 KB", start);

    transcript.begin_reply(start);

    assert_eq!(texts(&transcript), Vec::<Option<&str>>::new());
    assert_eq!(transcript.ghost().map(Ghost::question), Some(0..2));
    assert_eq!(transcript.end_row(), 2);
}

#[test]
fn has_no_ghost_when_no_question_comes_before_the_reply() {
    let start = Instant::now();
    let mut alone = Transcript::default();
    let mut after_a_notice = transcript(&["hello"], start);
    after_a_notice.notice("image-unsupported", start);

    alone.begin_reply(start);
    after_a_notice.begin_reply(start);

    assert_eq!(alone.ghost(), None);
    assert_eq!(after_a_notice.ghost(), None);
}

#[test]
fn the_reply_rolls_in_while_the_question_leaves() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);

    transcript.hear(Kind::Answer, "Hi", after(start, 700));

    assert!(transcript.is_animating(after(start, 1_800)));
    assert!(!transcript.is_animating(after(start, 1_900)));
}

#[test]
fn the_question_leaving_at_the_end_then_fading_in_keeps_the_window_moving() {
    let start = Instant::now();
    let mut transcript = asked(&["how are you"], start);
    transcript.hear(Kind::Answer, "Hi", start);

    transcript.end_reply(None, after(start, 2_000));

    assert!(transcript.is_animating(after(start, 2_599)));
    assert!(!transcript.is_animating(after(start, 2_600)));
}

#[test]
fn shows_its_caret_only_once_the_question_is_gone_and_no_tool_runs() {
    let start = Instant::now();
    let mut transcript = asked(&["how"], start);
    let waiting = transcript.shows_caret(start);
    transcript.hear(Kind::Answer, "Well, ", start);
    let rolling = transcript.shows_caret(after(start, 100));
    let settled = transcript.shows_caret(after(start, 300));
    transcript.start_tool(
        "t1".into(),
        crate::transcript::tool::Tool {
            glyph: crate::transcript::Glyph::Read,
            label: "a".to_owned(),
        },
        after(start, 300),
    );

    assert!(!waiting);
    assert!(!rolling);
    assert!(settled);
    assert!(!transcript.shows_caret(after(start, 400)));
}
