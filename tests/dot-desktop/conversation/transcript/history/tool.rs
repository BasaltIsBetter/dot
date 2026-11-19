use super::*;
use crate::conversation::transcript::fade::HOTTEST;
use crate::conversation::transcript::ghost::Ghost;
use crate::conversation::transcript::history::tests::{VISIBLE, after, texts, transcript};

fn replying(now: Instant) -> Transcript {
    let mut transcript = transcript(&["hello"], now);
    transcript.begin_reply(VISIBLE, now);
    transcript
}

fn heat_of(transcript: &Transcript, position: usize, now: Instant) -> u8 {
    let entry = transcript.entry(position).unwrap();
    entry.heat(entry.arrivals.last(), now)
}

#[test]
fn a_first_tool_takes_the_place_of_the_waiting_row() {
    let now = Instant::now();
    let mut transcript = replying(now);

    transcript.start_tool("t1".into(), "tool-run ls", VISIBLE, now);

    assert_eq!(
        texts(&transcript),
        vec![Some("hello"), Some("<gap>"), Some("tool-run ls")]
    );
    assert_eq!(transcript.entry(1).unwrap().kind, Kind::Tool);
}

#[test]
fn tools_that_follow_each_other_have_no_blank_row_between_them() {
    let now = Instant::now();
    let mut transcript = replying(now);

    transcript.hear(Kind::Answer, "Let me look.", VISIBLE, now);
    transcript.start_tool("t1".into(), "tool-read a.rs", VISIBLE, now);
    transcript.start_tool("t2".into(), "tool-read b.rs", VISIBLE, now);
    transcript.hear(Kind::Answer, "Done.", VISIBLE, now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("hello"),
            Some("<gap>"),
            Some("Let me look."),
            Some("<gap>"),
            Some("tool-read a.rs"),
            Some("tool-read b.rs"),
            Some("<gap>"),
            Some("Done."),
        ]
    );
}

#[test]
fn a_running_tool_stays_orange_until_it_ends_then_cools() {
    let now = Instant::now();
    let mut transcript = replying(now);
    transcript.start_tool("t1".into(), "tool-run sleep 5", VISIBLE, now);

    assert_eq!(heat_of(&transcript, 1, after(now, 5_000)), HOTTEST);

    let ended = after(now, 5_000);
    transcript.end_tool("t1", "  tool-exit 0", VISIBLE, ended);

    assert_eq!(texts(&transcript)[2], Some("tool-run sleep 5  tool-exit 0"));
    assert_eq!(heat_of(&transcript, 1, ended), HOTTEST);
    let cooling = heat_of(&transcript, 1, after(now, 5_600));
    assert!(cooling > 0 && cooling < HOTTEST, "{cooling}");
    assert_eq!(heat_of(&transcript, 1, after(now, 6_200)), 0);
    assert!(transcript.is_fresh(after(now, 6_199)));
}

#[test]
fn ends_the_tool_it_is_told_about_and_ignores_an_unknown_one() {
    let now = Instant::now();
    let mut transcript = replying(now);
    transcript.start_tool("t1".into(), "tool-read a.rs", VISIBLE, now);
    transcript.start_tool("t2".into(), "tool-read b.rs", VISIBLE, now);

    transcript.end_tool("t1", "  3 tool-lines", VISIBLE, now);
    transcript.end_tool("unknown", "  tool-failed", VISIBLE, now);
    transcript.end_tool("t1", "  again", VISIBLE, now);

    assert_eq!(
        texts(&transcript),
        vec![
            Some("hello"),
            Some("<gap>"),
            Some("tool-read a.rs  3 tool-lines"),
            Some("tool-read b.rs"),
        ]
    );
    assert!(transcript.entry(1).unwrap().ended.is_some());
    assert!(transcript.entry(2).unwrap().ended.is_none());
}

#[test]
fn rows_below_a_tool_that_wraps_when_it_ends_move_down() {
    let now = Instant::now();
    let mut transcript = replying(now);
    transcript.start_tool("t1".into(), "tool-run ls", VISIBLE, now);
    transcript.start_tool("t2".into(), "tool-run pwd", VISIBLE, now);

    let long = format!("  {}", "x".repeat(200));
    transcript.end_tool("t1", &long, VISIBLE, now);

    let shown = texts(&transcript);
    assert!(shown.len() > 4, "{shown:?}");
    assert_eq!(shown.last(), Some(&Some("tool-run pwd")));
    assert_ne!(shown[shown.len() - 2], Some("<gap>"));
}

#[test]
fn the_end_of_the_reply_settles_the_tools_still_running() {
    let now = Instant::now();
    let mut transcript = replying(now);
    transcript.start_tool("t1".into(), "tool-run sleep 60", VISIBLE, now);

    let stopped = after(now, 2_000);
    transcript.end_reply(Some("reply-stopped"), VISIBLE, stopped);
    transcript.end_tool("t1", "  tool-exit 0", VISIBLE, stopped);

    assert_eq!(transcript.entry(1).unwrap().ended, Some(stopped));
    assert_eq!(heat_of(&transcript, 1, after(now, 3_200)), 0);
    assert_eq!(texts(&transcript)[2], Some("tool-run sleep 60"));
}

#[test]
fn hears_no_tool_outside_a_reply() {
    let now = Instant::now();
    let mut transcript = transcript(&["hello"], now);

    transcript.start_tool("t1".into(), "tool-run ls", VISIBLE, now);

    assert_eq!(texts(&transcript), vec![Some("hello")]);
}

#[test]
fn a_tool_in_place_of_the_question_takes_its_last_words_when_it_ends() {
    let now = Instant::now();
    let mut transcript = transcript(&["aa bb cc dd ee ff gg"], now);
    transcript.begin_reply(VISIBLE, now);

    transcript.start_tool("t1".into(), "tool-run ls", VISIBLE, now);
    let waiting = transcript.ghost().is_some_and(|ghost| !ghost.is_complete());
    transcript.end_tool("t1", "  tool-exit 0", VISIBLE, now);

    assert!(waiting);
    assert!(transcript.ghost().is_some_and(Ghost::is_complete));
    assert_eq!(texts(&transcript)[0], Some("aa bb cc dd ee ff gg"));
}
