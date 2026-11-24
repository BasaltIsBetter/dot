use std::time::Duration;

use super::*;
use crate::code::{Change, ChangeHunk, ChangeLine, CodeBlock};
use crate::line::columns;
use crate::transcript::entry::Kind;
use crate::transcript::tool::Tool;

fn width() -> TextWidth {
    TextWidth::default()
}

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn tool(glyph: Glyph) -> Tool {
    Tool {
        glyph,
        label: "cargo test".to_owned(),
    }
}

fn whole(line: &str) -> Aside<'_> {
    Aside {
        cut: "",
        kept: line,
    }
}

fn summed_up(glyph: Glyph, line: &str, now: Instant) -> Entry {
    let mut entry = Entry::tool(tool(glyph), now);
    entry.end(now);
    entry.sum_up(whole(line), now, width());
    entry
}

const LONG: Aside<'static> = Aside {
    cut: "⌘ cargo test --workspace --all-features",
    kept: "  ✓ · 0,8 s",
};

fn change() -> Change {
    Change {
        path: "src/a.rs".to_owned(),
        created: false,
        hunks: vec![ChangeHunk {
            new_start: 1,
            lines: vec![ChangeLine::Added("new".to_owned())],
        }],
        neighbours: None,
    }
}

#[test]
fn only_a_command_runs_a_command() {
    let now = Instant::now();

    assert!(Entry::tool(tool(Glyph::Command), now).runs_a_command());
    assert!(!Entry::tool(tool(Glyph::Read), now).runs_a_command());
    assert!(!Entry::tool(tool(Glyph::Agents), now).runs_a_command());
    assert!(!Entry::new(Kind::Answer, "cargo test", now, width()).runs_a_command());
}

#[test]
fn only_a_command_with_its_line_sums_one_up() {
    let now = Instant::now();
    let mut with_code = summed_up(Glyph::Command, "⌘ cargo test  ✓", now);
    with_code.show_code(CodeBlock::new(change(), now).unwrap());

    assert!(summed_up(Glyph::Command, "⌘ cargo test  ✓", now).sums_up_a_command());
    assert!(!Entry::tool(tool(Glyph::Command), now).sums_up_a_command());
    assert!(!summed_up(Glyph::Agents, "⧉ explore · 12 files", now).sums_up_a_command());
    assert!(!with_code.sums_up_a_command());
}

#[test]
fn replaces_its_line_and_keeps_the_one_before_for_the_motion() {
    let start = Instant::now();
    let mut entry = summed_up(Glyph::Command, "⌘ a  ✓ · 0,1 s", start);

    let replaced = entry.replace_aside(whole("⌘ b  ✓ · 0,2 s"), after(start, 500), width());

    assert!(replaced);
    assert_eq!(entry.text, "⌘ b  ✓ · 0,2 s");
    assert_eq!(entry.row_count(), 1);
    assert_eq!(entry.arrivals, []);
    assert_eq!(
        entry.replaced(),
        Some(&Replaced {
            before: "⌘ a  ✓ · 0,1 s".to_owned(),
            at: after(start, 500),
        })
    );
}

#[test]
fn wraps_the_line_that_replaced_the_other_at_the_width() {
    let now = Instant::now();
    let mut entry = summed_up(Glyph::Command, "⌘ a  ✓", now);

    let line = "⌘ cargo test --workspace  ✓ · 12,4 s";
    entry.replace_aside(whole(line), now, TextWidth::new(10));

    assert_eq!(entry.row_count(), crate::wrap::wrap(line, 10).len());
    assert!(entry.row_count() > 1);
}

#[test]
fn keeps_its_line_when_the_new_one_shows_nothing() {
    let now = Instant::now();
    let mut entry = summed_up(Glyph::Command, "⌘ a  ✓", now);

    assert!(!entry.replace_aside(whole(""), now, width()));
    assert!(!entry.replace_aside(whole("\u{7}"), now, width()));

    assert_eq!(entry.text, "⌘ a  ✓");
    assert_eq!(entry.replaced(), None);
}

#[test]
fn a_tool_that_never_summed_itself_up_has_no_line_to_replace() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(Glyph::Command), now);

    assert!(!entry.replace_aside(whole("⌘ b  ✓"), now, width()));

    assert_eq!(entry.text, "");
}

#[test]
fn reuses_the_room_of_the_line_it_replaced_for_the_next_one() {
    let start = Instant::now();
    let mut entry = summed_up(Glyph::Command, "⌘ a  ✓ · 0,1 s", start);
    entry.replace_aside(whole("⌘ b  ✓ · 0,2 s"), after(start, 10), width());

    entry.replace_aside(whole("⌘ c  ✓ · 0,3 s"), after(start, 20), width());

    let replaced = entry.replaced().unwrap();
    assert_eq!(entry.text, "⌘ c  ✓ · 0,3 s");
    assert_eq!(replaced.before, "⌘ b  ✓ · 0,2 s");
    assert_eq!(replaced.at, after(start, 20));
    assert_eq!(entry.row_count(), 1);
    assert_eq!(entry.row(0), 0..entry.text.len());
}

#[test]
fn an_entry_without_a_tool_has_no_line_to_replace() {
    let now = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "hello", now, width());

    assert!(!entry.replace_aside(whole("⌘ b  ✓"), now, width()));

    assert_eq!(entry.text, "hello");
    assert_eq!(entry.replaced(), None);
}

#[test]
fn cuts_the_command_so_that_the_line_holds_on_one_row_with_its_measure_whole() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(Glyph::Command), now);

    entry.sum_up(LONG, now, TextWidth::new(20));

    assert_eq!(entry.text, "⌘ cargo …  ✓ · 0,8 s");
    assert_eq!(columns(&entry.text), 20);
    assert_eq!(entry.row_count(), 1);
}

#[test]
fn keeps_a_command_that_fits_whole() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(Glyph::Command), now);

    entry.sum_up(LONG, now, TextWidth::new(50));

    assert_eq!(
        entry.text,
        "⌘ cargo test --workspace --all-features  ✓ · 0,8 s"
    );
    assert_eq!(columns(&entry.text), 50);
}

#[test]
fn cuts_the_command_again_each_time_the_width_changes() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(Glyph::Command), now);
    entry.sum_up(LONG, now, width());

    entry.rewrap(TextWidth::new(20));
    let narrow = (entry.text.clone(), entry.row_count());
    entry.rewrap(width());

    assert_eq!(narrow, ("⌘ cargo …  ✓ · 0,8 s".to_owned(), 1));
    assert_eq!(
        entry.text,
        "⌘ cargo test --workspace --all-features  ✓ · 0,8 s"
    );
}

#[test]
fn a_line_that_replaced_another_is_cut_again_when_the_width_changes() {
    let now = Instant::now();
    let mut entry = summed_up(Glyph::Command, "⌘ a  ✓", now);
    entry.replace_aside(LONG, now, width());

    entry.rewrap(TextWidth::new(20));

    assert_eq!(entry.text, "⌘ cargo …  ✓ · 0,8 s");
}

#[test]
fn a_measure_wider_than_the_row_leaves_no_room_for_the_command_and_wraps() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(Glyph::Command), now);

    entry.sum_up(LONG, now, TextWidth::new(8));

    assert_eq!(entry.text, "  ✓ · 0,8 s");
    assert_eq!(entry.row_count(), 2);
}

#[test]
fn only_a_tool_sums_itself_up() {
    let now = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "hello", now, width());

    assert!(!entry.sum_up(LONG, now, width()));

    assert_eq!(entry.text, "hello");
    assert_eq!(entry.summary(), None);
}
