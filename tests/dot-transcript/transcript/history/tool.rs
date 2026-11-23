use std::time::{Duration, Instant};

use crate::transcript::Glyph;
use crate::transcript::history::tests::text_of;
use crate::transcript::tool::Tool;

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn read(label: &str) -> Tool {
    Tool {
        glyph: Glyph::Read,
        label: label.to_owned(),
    }
}

fn command(label: &str) -> Tool {
    Tool {
        glyph: Glyph::Command,
        label: label.to_owned(),
    }
}

fn replying(question: &str, now: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    transcript.append("earlier", now);
    transcript.append(question, now);
    transcript.begin_reply(now);
    transcript
}

fn rows_of(transcript: &Transcript) -> Vec<Option<&str>> {
    (0..transcript.total())
        .map(|index| text_of(transcript.row(index)))
        .collect()
}

fn gutter_rows(transcript: &Transcript) -> Vec<(usize, String)> {
    transcript
        .tools_from(0)
        .map(|(row, entry)| (row, entry.mark.as_ref().unwrap().tool.label.clone()))
        .collect()
}

#[test]
fn a_tool_takes_no_row_of_the_reply() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.hear(Kind::Answer, "Let me read it.", start);

    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));
    transcript.hear(Kind::Answer, " ", after(start, 20));

    assert_eq!(
        rows_of(&transcript),
        vec![
            Some("earlier"),
            Some("<gap>"),
            Some("look"),
            Some("<gap>"),
            Some("Let me read it."),
            Some("<gap>"),
            Some(" "),
        ]
    );
}

#[test]
fn a_tool_is_marked_on_the_row_claude_called_it_from() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.hear(Kind::Answer, "Let me read it.", start);

    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));
    transcript.start_tool("t2".into(), read("mod.rs"), after(start, 20));

    assert_eq!(
        gutter_rows(&transcript),
        vec![(4, "layout.rs".to_owned()), (4, "mod.rs".to_owned())]
    );
}

#[test]
fn a_first_tool_is_marked_on_the_row_the_reply_starts_on() {
    let start = Instant::now();
    let mut transcript = replying("look at this", start);

    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));

    assert_eq!(transcript.anchor(), Some(2));
    assert_eq!(gutter_rows(&transcript), vec![(2, "layout.rs".to_owned())]);
}

#[test]
fn a_first_tool_stays_on_the_first_row_of_the_reply_once_written() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));

    transcript.hear(Kind::Answer, "Read it.", after(start, 20));
    transcript.end_reply(None, after(start, 30));

    let reply_row = rows_of(&transcript)
        .iter()
        .position(|text| *text == Some("Read it."))
        .unwrap();
    assert_eq!(
        gutter_rows(&transcript),
        vec![(reply_row, "layout.rs".to_owned())]
    );
}

#[test]
fn a_command_that_ends_sums_itself_up_on_its_own_row() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Running the tests.", start);
    transcript.start_tool("t1".into(), command("cargo test"), after(start, 10));

    transcript.end_tool(
        "t1",
        "✓".to_owned(),
        Some("⌘ cargo test  ✓ · 0,8 s"),
        after(start, 810),
    );

    let rows = rows_of(&transcript);
    assert_eq!(rows[rows.len() - 1], Some("⌘ cargo test  ✓ · 0,8 s"));
    assert_eq!(rows[rows.len() - 2], Some("<gap>"));
    let (row, _) = transcript.tools_from(0).next().unwrap();
    assert_eq!(row, transcript.total() - 1);
}

#[test]
fn two_sums_in_a_row_touch() {
    let start = Instant::now();
    let mut transcript = replying("test it", start);
    transcript.hear(Kind::Answer, "Two commands.", start);
    transcript.start_tool("t1".into(), command("a"), start);
    transcript.end_tool("t1", "✓".to_owned(), Some("⌘ a  ✓"), start);
    transcript.start_tool("t2".into(), command("b"), start);

    transcript.end_tool("t2", "✓".to_owned(), Some("⌘ b  ✓"), start);

    let rows = rows_of(&transcript);
    assert_eq!(&rows[rows.len() - 2..], [Some("⌘ a  ✓"), Some("⌘ b  ✓")]);
}

#[test]
fn ending_a_tool_keeps_its_result_for_the_gutter() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("layout.rs"), start);

    let running = transcript
        .running_tool("t1")
        .map(|(tool, at)| (tool.clone(), at));
    transcript.end_tool("t1", "212 l.".to_owned(), None, after(start, 300));

    assert_eq!(running, Some((read("layout.rs"), start)));
    assert!(transcript.running_tool("t1").is_none());
    let (_, entry) = transcript.tools_from(0).next().unwrap();
    assert_eq!(
        entry.mark.as_ref().unwrap().result.as_deref(),
        Some("212 l.")
    );
    assert_eq!(entry.ended, Some(after(start, 300)));
}

#[test]
fn an_unknown_or_ended_tool_changes_nothing() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("layout.rs"), start);
    transcript.end_tool("t1", "1 l.".to_owned(), None, start);

    transcript.end_tool("t1", "2 l.".to_owned(), Some("again"), start);
    transcript.end_tool("nope", "3 l.".to_owned(), Some("never"), start);

    let (_, entry) = transcript.tools_from(0).next().unwrap();
    assert_eq!(entry.mark.as_ref().unwrap().result.as_deref(), Some("1 l."));
    assert_eq!(entry.text, "");
}

#[test]
fn no_tool_starts_outside_a_reply() {
    let start = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello", start);

    transcript.start_tool("t1".into(), read("layout.rs"), start);

    assert_eq!(transcript.tools_from(0).count(), 0);
}

#[test]
fn the_end_of_the_reply_ends_the_tools_still_running() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("layout.rs"), start);

    transcript.end_reply(None, after(start, 50));

    let (_, entry) = transcript.tools_from(0).next().unwrap();
    assert_eq!(entry.ended, Some(after(start, 50)));
    assert!(transcript.running_tool("t1").is_none());
}

#[test]
fn tools_from_a_row_start_with_the_run_of_tools_above_it() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.hear(Kind::Answer, "Reading.", start);
    for (index, name) in ["a.rs", "b.rs", "c.rs"].iter().enumerate() {
        transcript.start_tool(format!("t{index}"), read(name), start);
    }
    transcript.hear(Kind::Answer, " ", start);
    let below = transcript.total() - 1;

    let labels: Vec<_> = transcript
        .tools_from(below)
        .map(|(_, entry)| entry.mark.as_ref().unwrap().tool.label.clone())
        .collect();

    assert_eq!(labels, ["a.rs", "b.rs", "c.rs"]);
}

#[test]
fn a_reply_of_tools_only_marks_them_on_the_last_row_before_it() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("layout.rs"), start);

    transcript.end_reply(None, start);

    assert_eq!(gutter_rows(&transcript), vec![(2, "layout.rs".to_owned())]);
}

#[test]
fn a_tool_between_two_texts_is_marked_on_the_text_before_it() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.hear(Kind::Answer, "Reading.", start);
    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));

    transcript.hear(Kind::Answer, "Done.", after(start, 20));

    assert_eq!(gutter_rows(&transcript), vec![(4, "layout.rs".to_owned())]);
}

#[test]
fn a_first_tool_after_an_earlier_reply_is_marked_where_the_new_reply_starts() {
    let start = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("earlier", start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Sure.", start);
    transcript.end_reply(None, start);
    transcript.append("look", start);
    transcript.begin_reply(start);

    transcript.start_tool("t1".into(), read("layout.rs"), after(start, 10));

    assert_eq!(transcript.anchor(), Some(4));
    assert_eq!(gutter_rows(&transcript), vec![(4, "layout.rs".to_owned())]);
}

#[test]
fn lists_the_tools_still_running_in_the_order_they_started() {
    let start = Instant::now();
    let mut transcript = replying("look", start);
    transcript.start_tool("t1".into(), read("a.rs"), start);
    transcript.start_tool("t2".into(), command("ls"), start);
    transcript.start_tool("t3".into(), read("b.rs"), start);

    transcript.end_tool("t2", "✓".into(), None, after(start, 10));

    let running: Vec<&str> = transcript
        .running_tools()
        .map(|tool| tool.label.as_str())
        .collect();
    assert_eq!(running, ["a.rs", "b.rs"]);
}

#[test]
fn keeps_the_last_lines_a_running_tool_printed() {
    let now = Instant::now();
    let mut transcript = replying("run it", now);
    transcript.start_tool("toolu_1".to_owned(), command("cargo test"), now);

    transcript.tool_output("toolu_1", vec!["Compiling".to_owned()]);
    transcript.tool_output(
        "toolu_1",
        vec!["Compiling".to_owned(), "Finished".to_owned()],
    );
    transcript.tool_output("toolu_9", vec!["lost".to_owned()]);

    let running: Vec<_> = transcript.running().collect();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, "toolu_1");
    assert_eq!(running[0].tool.label, "cargo test");
    assert_eq!(running[0].started, now);
    assert_eq!(running[0].output, ["Compiling", "Finished"]);
}

#[test]
fn an_ended_tool_runs_no_more_and_keeps_no_output() {
    let now = Instant::now();
    let mut transcript = replying("list", now);
    transcript.start_tool("toolu_1".to_owned(), command("ls"), now);
    transcript.end_tool("toolu_1", "✓".to_owned(), None, now);

    transcript.tool_output("toolu_1", vec!["late".to_owned()]);

    assert_eq!(transcript.running().count(), 0);
}

#[test]
fn stands_each_glyph_for_a_character() {
    let marks: String = [
        Glyph::Read,
        Glyph::Search,
        Glyph::Edit,
        Glyph::Create,
        Glyph::Command,
        Glyph::Agents,
        Glyph::Web,
        Glyph::Other,
    ]
    .into_iter()
    .map(Glyph::mark)
    .collect();

    assert_eq!(marks, "▤⌕✎✚⌘⧉◎◇");
}

fn edit(label: &str) -> Tool {
    Tool {
        glyph: Glyph::Edit,
        label: label.to_owned(),
    }
}

fn change_of(path: &str) -> crate::code::Change {
    let mut lines: Vec<crate::code::ChangeLine> = (1..=8)
        .map(|line| crate::code::ChangeLine::Same(format!("line {line}")))
        .collect();
    lines.push(crate::code::ChangeLine::Added("new".to_owned()));
    crate::code::Change {
        path: path.to_owned(),
        created: false,
        hunks: vec![crate::code::ChangeHunk {
            new_start: 1,
            lines,
        }],
        neighbours: None,
    }
}

#[test]
fn shows_the_change_of_a_running_tool_in_rows_of_the_reply() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.hear(Kind::Answer, "Fixing.", start);
    transcript.start_tool("t1".into(), edit("a.rs"), after(start, 10));
    let before = transcript.total();

    transcript.change_tool("t1", change_of("src/a.rs"), after(start, 20));
    transcript.change_tool("unknown", change_of("src/b.rs"), after(start, 20));

    assert_eq!(transcript.total(), before + 1 + 5);
    assert_eq!(gutter_rows(&transcript)[0], (before + 1, "a.rs".to_owned()));
    assert!(transcript.is_animating(after(start, 20 + 70 * 4 + 599)));
}

#[test]
fn shows_nothing_for_an_ended_tool_or_an_empty_change() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.start_tool("t2".into(), edit("b.rs"), start);
    transcript.end_tool("t1", "+1".to_owned(), None, after(start, 10));
    let before = transcript.total();

    transcript.change_tool("t1", change_of("src/a.rs"), after(start, 20));
    transcript.change_tool("t2", crate::code::Change::default(), after(start, 20));

    assert_eq!(transcript.total(), before);
}

#[test]
fn opens_and_closes_a_fold_where_it_is_clicked() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.change_tool("t1", change_of("src/a.rs"), start);
    let name = transcript.total() - 5;

    assert!(!transcript.toggle_code(name + 2));
    assert!(!transcript.toggle_code(0));
    assert!(!transcript.toggle_code(transcript.total() + 3));
    assert!(transcript.toggle_code(name + 1));
    assert_eq!(transcript.total(), name + 10);
    assert!(transcript.toggle_code(name + 4));
    assert_eq!(transcript.total(), name + 5);
}

#[test]
fn opens_the_file_from_the_name_of_its_change() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.change_tool("t1", change_of("src/a.rs"), start);
    let name = transcript.total() - 5;

    assert_eq!(transcript.file_at(name, start), Some("src/a.rs"));
    assert_eq!(transcript.file_at(name + 1, start), None);
    assert_eq!(transcript.file_at(0, start), None);
    assert_eq!(transcript.file_at(transcript.total() + 1, start), None);
}

#[test]
fn keeps_one_row_per_line_of_code_whatever_the_width() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.change_tool("t1", change_of("src/a.rs"), start);
    let total = transcript.total();

    transcript.rewrap(crate::line::TextWidth::new(8), 0.0, start);

    assert!(transcript.total() >= total);
    let name = (0..transcript.total())
        .find(|&row| transcript.file_at(row, start).is_some())
        .unwrap();
    assert_eq!(transcript.total() - name, 5);
}

#[test]
fn stops_animating_once_the_change_has_settled() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.change_tool("t1", change_of("src/a.rs"), after(start, 5_000));

    assert!(transcript.is_animating(after(start, 5_000 + 879)));
    assert!(!transcript.is_animating(after(start, 5_000 + 880)));
}

#[test]
fn leaves_a_blank_row_around_a_change_but_none_between_two_asides() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    transcript.start_tool("t1".into(), edit("a.rs"), start);
    transcript.change_tool("t1", change_of("src/a.rs"), start);
    let after_first = transcript.total();
    transcript.start_tool("t2".into(), edit("b.rs"), start);
    transcript.change_tool("t2", change_of("src/b.rs"), start);
    let after_second = transcript.total();
    transcript.start_tool("t3".into(), command("ls"), start);
    transcript.end_tool("t3", "✓".to_owned(), Some("⌘ ls"), start);
    transcript.start_tool("t4".into(), command("pwd"), start);
    transcript.end_tool("t4", "✓".to_owned(), Some("⌘ pwd"), start);

    assert_eq!(after_second, after_first + 1 + 5);
    assert_eq!(transcript.total(), after_second + 1 + 1 + 1);
}
