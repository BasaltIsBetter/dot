use std::time::Instant;

use crate::line::TextWidth;
use crate::transcript::entry::Kind;
use crate::transcript::history::tool::tests::{
    after, agents, change_of, command, edit, gutter_rows, read, replying, rows_of, whole,
};
use crate::transcript::tool::Tool;

use super::*;

fn run(transcript: &mut Transcript, id: &str, tool: Tool, aside: &str, now: Instant) {
    transcript.start_tool(id.into(), tool, now);
    transcript.end_tool(id, "✓".to_owned(), Some(whole(aside)), now);
}

fn last_rows(transcript: &Transcript, count: usize) -> Vec<Option<&str>> {
    let rows = rows_of(transcript);
    rows[rows.len() - count..].to_vec()
}

fn holding<'a>(transcript: &'a Transcript, text: &str) -> &'a Entry {
    transcript
        .entries
        .iter()
        .find(|entry| entry.text == text)
        .unwrap()
}

#[test]
fn a_command_right_after_another_takes_its_row() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Two commands.", start);
    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    let total = transcript.total();

    run(
        &mut transcript,
        "t2",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );

    assert_eq!(transcript.total(), total);
    assert_eq!(last_rows(&transcript, 2), [Some("<gap>"), Some("⌘ b  ✓")]);
    let replaced = holding(&transcript, "⌘ b  ✓").replaced().unwrap();
    assert_eq!(replaced.before, "⌘ a  ✓");
    assert_eq!(replaced.at, after(start, 10));
    assert!(transcript.is_fresh(after(start, 1_000)));
}

#[test]
fn commands_in_a_row_keep_one_row_the_last_one_to_end_on_it() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Three commands.", start);

    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    transcript.start_tool("t2".into(), read("lib.rs"), after(start, 5));
    transcript.end_tool("t2", "12 l.".to_owned(), None, after(start, 6));
    run(
        &mut transcript,
        "t3",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );
    run(
        &mut transcript,
        "t4",
        command("c"),
        "⌘ c  ✓",
        after(start, 20),
    );

    assert_eq!(
        last_rows(&transcript, 3),
        [Some("Three commands."), Some("<gap>"), Some("⌘ c  ✓")]
    );
    let row = transcript.total() - 1;
    let gutter = gutter_rows(&transcript);
    let commands: Vec<(usize, String)> = gutter
        .into_iter()
        .filter(|(_, label)| label.len() == 1)
        .collect();
    assert_eq!(
        commands,
        [
            (row, "a".to_owned()),
            (row, "b".to_owned()),
            (row, "c".to_owned())
        ]
    );
}

#[test]
fn a_line_that_takes_more_rows_than_the_one_it_replaced_moves_what_follows() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.rewrap(TextWidth::new(10), 0.0, start);
    transcript.hear(Kind::Answer, "Go.", start);
    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    let total = transcript.total();

    run(
        &mut transcript,
        "t2",
        command("b"),
        "⌘ cargo test  ✓",
        after(start, 10),
    );

    assert_eq!(transcript.total(), total + 1);
    assert_eq!(
        last_rows(&transcript, 2),
        [Some("⌘ cargo "), Some("test  ✓")]
    );
}

#[test]
fn text_between_two_commands_keeps_both_lines() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "First.", start);
    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    transcript.hear(Kind::Answer, "Then.", after(start, 5));

    run(
        &mut transcript,
        "t2",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );

    assert_eq!(
        last_rows(&transcript, 5),
        [
            Some("⌘ a  ✓"),
            Some("<gap>"),
            Some("Then."),
            Some("<gap>"),
            Some("⌘ b  ✓")
        ]
    );
}

#[test]
fn a_change_between_two_commands_keeps_both_lines() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.hear(Kind::Answer, "Fixing.", start);
    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    transcript.start_tool("t2".into(), edit("a.rs"), after(start, 5));
    transcript.change_tool("t2", change_of("src/a.rs"), after(start, 5));
    transcript.end_tool("t2", "+1 −0".to_owned(), None, after(start, 6));

    run(
        &mut transcript,
        "t3",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );

    let rows = rows_of(&transcript);
    assert!(rows.contains(&Some("⌘ a  ✓")), "{rows:?}");
    assert_eq!(rows.last(), Some(&Some("⌘ b  ✓")));
}

#[test]
fn a_command_never_takes_the_line_of_an_agent_nor_an_agent_that_of_a_command() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Mixed.", start);

    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    run(
        &mut transcript,
        "t2",
        agents("explore"),
        "⧉ explore · 3 s",
        after(start, 5),
    );
    run(
        &mut transcript,
        "t3",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );

    assert_eq!(
        last_rows(&transcript, 3),
        [Some("⌘ a  ✓"), Some("⧉ explore · 3 s"), Some("⌘ b  ✓")]
    );
}

#[test]
fn a_command_that_ends_before_one_started_earlier_shares_its_row() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Two at once.", start);
    transcript.start_tool("t1".into(), command("a"), start);
    transcript.start_tool("t2".into(), command("b"), start);
    transcript.end_tool(
        "t2",
        "✓".to_owned(),
        Some(whole("⌘ b  ✓")),
        after(start, 10),
    );

    transcript.end_tool(
        "t1",
        "✓".to_owned(),
        Some(whole("⌘ a  ✓")),
        after(start, 20),
    );

    assert_eq!(
        last_rows(&transcript, 3),
        [Some("Two at once."), Some("<gap>"), Some("⌘ a  ✓")]
    );
}

#[test]
fn a_command_never_takes_the_line_of_an_earlier_reply() {
    let start = Instant::now();
    let mut transcript = replying("first", start);
    transcript.start_tool("t1".into(), command("a"), start);
    transcript.end_tool("t1", "✓".to_owned(), Some(whole("⌘ a  ✓")), start);
    transcript.end_reply(None, start);
    transcript.append("second", after(start, 5));
    transcript.begin_reply(after(start, 5));

    run(
        &mut transcript,
        "t2",
        command("b"),
        "⌘ b  ✓",
        after(start, 10),
    );

    let rows = rows_of(&transcript);
    assert!(rows.contains(&Some("⌘ a  ✓")), "{rows:?}");
    assert!(rows.contains(&Some("⌘ b  ✓")), "{rows:?}");
}

#[test]
fn a_line_replaced_after_it_settled_settles_again_once_cold() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Slow commands.", start);
    run(&mut transcript, "t1", command("a"), "⌘ a  ✓", start);
    transcript.start_tool("t2".into(), command("b"), after(start, 2_000));
    transcript.end_tool(
        "t2",
        "✓".to_owned(),
        Some(whole("⌘ b  ✓")),
        after(start, 3_000),
    );
    let replaced = holding(&transcript, "⌘ b  ✓").replaced().is_some();

    transcript.start_tool("t3".into(), command("c"), after(start, 5_000));

    assert!(replaced);
    assert_eq!(holding(&transcript, "⌘ b  ✓").replaced(), None);
}
