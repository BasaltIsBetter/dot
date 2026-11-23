use std::fmt::Write;

use super::*;

fn same(text: &str) -> PatchLine {
    PatchLine::Same(text.to_owned())
}

fn removed(text: &str) -> PatchLine {
    PatchLine::Removed(text.to_owned())
}

fn added(text: &str) -> PatchLine {
    PatchLine::Added(text.to_owned())
}

fn numbered(count: usize) -> String {
    (1..=count).fold(String::new(), |mut text, line| {
        writeln!(text, "{line}").unwrap();
        text
    })
}

#[test]
fn keeps_eight_unchanged_lines_around_what_changed() {
    let before = numbered(20);
    let after = before.replace("10\n", "ten\n");

    let change = FileChange::between("a.rs", &before, &after, false);

    assert_eq!(change.path, "a.rs");
    assert!(!change.created);
    assert_eq!(change.hunks.len(), 1);
    let hunk = &change.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (2, 2));
    assert_eq!(hunk.lines.len(), 8 + 2 + 8);
    assert_eq!(hunk.lines[0], same("2"));
    assert_eq!(hunk.lines[8], removed("10"));
    assert_eq!(hunk.lines[9], added("ten"));
    assert_eq!(hunk.lines[17], same("18"));
}

#[test]
fn starts_at_the_first_line_when_the_change_is_near_the_top() {
    let change = FileChange::between("a.rs", "a\nb\n", "A\nb\n", false);

    let hunk = &change.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (1, 1));
    assert_eq!(hunk.lines, [removed("a"), added("A"), same("b")]);
}

#[test]
fn counts_inserted_and_removed_lines_apart() {
    let inserted = FileChange::between("a.rs", "a\nc\n", "a\nb\nc\n", false);
    let removed_only = FileChange::between("a.rs", "a\nb\nc\n", "a\nc\n", false);

    assert_eq!(inserted.hunks[0].lines, [same("a"), added("b"), same("c")]);
    assert_eq!(
        removed_only.hunks[0].lines,
        [same("a"), removed("b"), same("c")]
    );
}

#[test]
fn adds_every_line_of_a_created_file_from_line_one() {
    let change = FileChange::between("new.rs", "", "a\nb\r\n", true);

    assert!(change.created);
    let hunk = &change.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (0, 1));
    assert_eq!(hunk.lines, [added("a"), added("b")]);
}

#[test]
fn has_no_hunk_when_nothing_changed() {
    let change = FileChange::between("a.rs", "a\nb\n", "a\nb\n", false);

    assert_eq!(change.hunks, []);
    assert_eq!(change.siblings, None);
}

fn patch(old_start: u64, new_start: u64, lines: Vec<PatchLine>) -> FileChange {
    FileChange {
        path: "a.rs".to_owned(),
        created: false,
        hunks: vec![Hunk {
            old_start,
            new_start,
            lines,
        }],
        siblings: None,
    }
}

#[test]
fn widens_a_hunk_with_the_lines_of_the_file_before_it() {
    let original = "a\nb\nc\nd\ne\nf\ng\nh\n";
    let change = patch(
        3,
        3,
        vec![
            same("c"),
            same("d"),
            same("e"),
            removed("f"),
            added("F"),
            same("g"),
            same("h"),
        ],
    );

    let widened = change.widened(original);

    let hunk = &widened.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (1, 1));
    assert_eq!(hunk.lines[..3], [same("a"), same("b"), same("c")]);
    assert_eq!(hunk.lines.len(), 9);
    assert_eq!(hunk.lines.last(), Some(&same("h")));
}

#[test]
fn widens_by_eight_lines_at_most_on_each_side() {
    let original = numbered(40);
    let change = patch(20, 20, vec![removed("20"), added("twenty")]);

    let widened = change.widened(&original);

    let hunk = &widened.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (12, 12));
    assert_eq!(hunk.lines.first(), Some(&same("12")));
    assert_eq!(hunk.lines.last(), Some(&same("28")));
    assert_eq!(hunk.lines.len(), 8 + 2 + 8);
}

#[test]
fn joins_hunks_that_come_to_touch_without_repeating_a_line() {
    let original = numbered(60);
    let change = FileChange {
        hunks: vec![
            Hunk {
                old_start: 10,
                new_start: 10,
                lines: vec![removed("10"), added("ten")],
            },
            Hunk {
                old_start: 22,
                new_start: 22,
                lines: vec![removed("22"), added("twenty-two")],
            },
            Hunk {
                old_start: 50,
                new_start: 50,
                lines: vec![removed("50"), added("fifty")],
            },
        ],
        ..patch(1, 1, Vec::new())
    };

    let widened = change.widened(&original);

    assert_eq!(widened.hunks.len(), 2);
    let joined = &widened.hunks[0];
    assert_eq!((joined.old_start, joined.new_start), (2, 2));
    assert_eq!(joined.lines.len(), 8 + 2 + 11 + 2 + 8);
    assert_eq!(
        joined
            .lines
            .iter()
            .filter(|line| **line == same("16"))
            .count(),
        1
    );
    let last = &widened.hunks[1];
    assert_eq!(last.old_start, 42);
    assert_eq!(last.lines.last(), Some(&same("58")));
}

#[test]
fn widens_an_insertion_from_the_line_it_comes_after() {
    let original = numbered(5);
    let change = patch(2, 3, vec![added("new")]);

    let widened = change.widened(&original);

    let hunk = &widened.hunks[0];
    assert_eq!(hunk.old_start, 1);
    assert_eq!(hunk.lines[..3], [same("1"), same("2"), added("new")]);
    assert_eq!(hunk.lines.last(), Some(&same("5")));
}

#[test]
fn never_reads_past_the_file_it_is_given() {
    let change = patch(9, 9, vec![same("x"), removed("y"), added("Y")]);

    let widened = change.widened("only\n");

    assert_eq!(widened.hunks[0].lines.len(), 3);
}

#[test]
fn joins_hunks_whose_context_meets_end_to_end() {
    let original = numbered(40);
    let change = FileChange {
        hunks: vec![
            Hunk {
                old_start: 10,
                new_start: 10,
                lines: vec![removed("10"), added("ten")],
            },
            Hunk {
                old_start: 27,
                new_start: 27,
                lines: vec![removed("27"), added("twenty-seven")],
            },
        ],
        ..patch(1, 1, Vec::new())
    };

    let widened = change.widened(&original);

    assert_eq!(widened.hunks.len(), 1);
    assert_eq!(widened.hunks[0].lines.len(), 8 + 2 + 8 + 8 + 2 + 8);
}
