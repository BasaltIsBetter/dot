use std::fmt::Write;

use proptest::prelude::*;

use super::*;
use crate::editor::tests::{added, removed, same};

fn hunk(new_start: u64, lines: Vec<PatchLine>) -> Hunk {
    Hunk {
        old_start: new_start,
        new_start,
        lines,
    }
}

fn versions() -> Versions {
    let change = vec![hunk(
        1,
        vec![same("a"), removed("b"), added("B"), same("c")],
    )];
    Versions::new("a\nB\nc".to_owned(), &[change], false)
}

#[test]
fn finds_the_file_before_claude_by_undoing_its_changes_newest_first() {
    let first = vec![hunk(2, vec![removed("b"), added("B")])];
    let second = vec![
        hunk(1, vec![added("top")]),
        hunk(4, vec![same("c"), added("d")]),
    ];

    let before = before_claude("top\na\nB\nc\nd", &[first, second]);

    assert_eq!(before.as_deref(), Some("a\nb\nc"));
    assert_eq!(
        before_claude("x", &[vec![hunk(1, vec![removed("y")])]]).as_deref(),
        Some("x\ny")
    );
    assert_eq!(
        before_claude("x", &[vec![hunk(0, vec![removed("y")])]]).as_deref(),
        Some("y\nx")
    );
}

#[test]
fn knows_no_before_when_the_file_changed_since() {
    let change = vec![hunk(1, vec![same("a"), added("B")])];

    assert_eq!(
        before_claude("a\nsomething else", std::slice::from_ref(&change)),
        None
    );
    assert_eq!(before_claude("", &[change]), None);
    assert_eq!(before_claude("a", &[vec![hunk(0, vec![added("a")])]]), None);
}

#[test]
fn steps_back_from_your_changes_to_claude_then_to_before_claude() {
    let mut versions = versions();
    let yours = "a\nB!\nc";

    assert_eq!(versions.back(yours), Some(Toward::Claude));
    versions.leave(yours);
    assert_eq!(versions.back("a\nB\nc"), Some(Toward::Before));
    assert_eq!(versions.text(Toward::Before), "a\nb\nc");
    assert_eq!(versions.back("a\nb\nc"), None);
}

#[test]
fn steps_forward_from_before_claude_to_claude_then_to_your_changes() {
    let mut versions = versions();

    assert_eq!(versions.forward("a\nb\nc"), Some(Toward::Claude));
    assert_eq!(versions.forward("a\nB\nc"), None);
    versions.leave("a\nB!\nc");
    assert_eq!(versions.forward("a\nB\nc"), Some(Toward::Yours));
    assert_eq!(versions.text(Toward::Yours), "a\nB!\nc");
    assert_eq!(versions.forward("a\nB!\nc"), None);
    versions.leave("a\nB\nc");
    assert_eq!(versions.text(Toward::Yours), "a\nB!\nc");
}

#[test]
fn never_steps_back_past_claude_in_a_file_it_created() {
    let versions = Versions::new("new\nfile".to_owned(), &[], true);

    assert_eq!(versions.back("new\nfile"), None);
    assert_eq!(versions.back("new\nfile!"), Some(Toward::Claude));
    assert_eq!(versions.forward("new\nfile"), None);
    assert!(versions.created());
    assert_eq!(versions.claude_count(), 2);
    assert!(!versions.by_claude("new"));
    assert_eq!(versions.text(Toward::Before), "new\nfile");
    assert_eq!(versions.text(Toward::Yours), "new\nfile");
}

#[test]
fn marks_the_lines_claude_changed() {
    let versions = versions();

    assert!(versions.by_claude("B"));
    assert!(!versions.by_claude("a"));
    assert_eq!(versions.claude_count(), 1);
}

#[test]
fn falls_back_on_the_lines_claude_added_when_before_is_unknown() {
    let change = vec![hunk(1, vec![same("x"), added("y"), added("z")])];

    let versions = Versions::new("changed since".to_owned(), &[change], false);

    assert!(versions.by_claude("y"));
    assert_eq!(versions.claude_count(), 2);
    assert_eq!(versions.back("changed since"), None);
}

#[test]
fn compares_lines_keeping_what_both_sides_share() {
    let lines = changed_lines("a\nb\nc\nd", "a\nB\nc\nnew\nd");

    assert_eq!(lines.changed, [false, true, false, true, false]);
    assert_eq!(lines.removed, 1);
    assert_eq!(lines.count(), 2);
    assert_eq!(
        changed_lines("same", "same"),
        Lines {
            changed: vec![false],
            removed: 0
        }
    );
    assert_eq!(changed_lines("a\nb\nc", "a").count(), 2);
    assert_eq!(versions().mine("a\nB\nc").count(), 0);
}

#[test]
fn marks_a_whole_middle_too_long_to_compare_as_changed() {
    let numbered = |kind: &str| {
        (0..1_001).fold(String::new(), |mut text, line| {
            writeln!(text, "{kind} {line}").unwrap();
            text
        })
    };
    let (before, after) = (numbered("old"), numbered("new"));

    let lines = changed_lines(&format!("top\n{before}"), &format!("top\n{after}"));

    assert!(!lines.changed[0]);
    assert!(lines.changed[1..1_002].iter().all(|&changed| changed));
    assert!(!lines.changed[1_002]);
    assert_eq!(lines.removed, 1_001);
}

#[test]
fn keeps_the_lines_that_did_not_move_and_marks_the_one_that_did() {
    assert_eq!(
        common(&["a", "b", "c"], &["b", "c", "a"]),
        [true, true, false]
    );
    assert_eq!(
        common(&["b", "c", "a"], &["a", "b", "c"]),
        [false, true, true]
    );
}

fn in_order(lines: &[&str], within: &[&str]) -> bool {
    let mut rest = within.iter();
    lines
        .iter()
        .all(|line| rest.any(|candidate| candidate == line))
}

fn longest_shared_run(old: &[&str], new: &[&str]) -> usize {
    (0_u32..1 << new.len())
        .filter_map(|chosen| {
            let picked: Vec<&str> = new
                .iter()
                .enumerate()
                .filter(|&(index, _)| (chosen >> index) & 1 == 1)
                .map(|(_, line)| *line)
                .collect();
            in_order(&picked, old).then_some(picked.len())
        })
        .max()
        .unwrap_or(0)
}

proptest! {
    #[test]
    fn keeps_a_longest_run_of_lines_both_sides_share(
        old in proptest::collection::vec("[abc]", 0..7),
        new in proptest::collection::vec("[abc]", 0..7),
    ) {
        let old: Vec<&str> = old.iter().map(String::as_str).collect();
        let new: Vec<&str> = new.iter().map(String::as_str).collect();

        let kept = common(&old, &new);

        let shared: Vec<&str> = new
            .iter()
            .zip(&kept)
            .filter(|&(_, &kept)| kept)
            .map(|(line, _)| *line)
            .collect();
        prop_assert!(in_order(&shared, &old));
        prop_assert_eq!(shared.len(), longest_shared_run(&old, &new));
    }
}
