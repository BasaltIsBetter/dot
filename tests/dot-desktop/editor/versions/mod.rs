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
    Versions::new("a\nB\nc".to_owned(), Some(&[change]), false)
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
    let versions = Versions::new("new\nfile".to_owned(), Some(&[]), true);

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

    let versions = Versions::new("changed since".to_owned(), Some(&[change]), false);

    assert!(versions.by_claude("y"));
    assert_eq!(versions.claude_count(), 2);
    assert_eq!(versions.back("changed since"), None);
}

#[test]
fn counts_none_of_your_lines_while_the_file_is_claude_s_version() {
    assert_eq!(versions().mine("a\nB\nc").count(), 0);
    assert_eq!(versions().mine("a\nB\nc\nd").count(), 1);
}
