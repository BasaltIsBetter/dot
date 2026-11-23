use super::*;
use crate::code::tests::{added, removed, same};

fn hunk(new_start: u64, lines: Vec<ChangeLine>) -> ChangeHunk {
    ChangeHunk { new_start, lines }
}

fn changed(number: u64, text: &str, old: &str, order: usize) -> Shown {
    Shown::Changed {
        number,
        text: text.to_owned(),
        old: old.to_owned(),
        order,
    }
}

fn same_at(number: u64, text: &str) -> Shown {
    Shown::Same {
        number,
        text: text.to_owned(),
    }
}

#[test]
fn pairs_removed_lines_with_the_added_lines_that_follow() {
    let edit = edit_of(vec![hunk(
        4,
        vec![
            removed("a"),
            removed("b"),
            added("A"),
            added("B"),
            added("C"),
        ],
    )]);

    assert_eq!(
        edit.lines,
        [
            changed(4, "A", "a", 0),
            changed(5, "B", "b", 1),
            Shown::Added {
                number: 6,
                text: "C".to_owned(),
                order: 2,
            },
        ]
    );
    assert_eq!(edit.changes, 3);
}

#[test]
fn keeps_a_removed_line_without_a_pair_and_gives_it_no_number() {
    let edit = edit_of(vec![hunk(
        1,
        vec![removed("a"), removed("b"), added("A"), same("c")],
    )]);

    assert_eq!(
        edit.lines,
        [
            changed(1, "A", "a", 0),
            Shown::Removed {
                old: "b".to_owned(),
                order: 1,
            },
            same_at(2, "c"),
        ]
    );
}

#[test]
fn ends_a_pairing_at_the_next_removed_line() {
    let edit = edit_of(vec![hunk(
        1,
        vec![
            removed("a"),
            removed("b"),
            added("A"),
            removed("c"),
            added("C"),
        ],
    )]);

    assert_eq!(
        edit.lines,
        [
            changed(1, "A", "a", 0),
            Shown::Removed {
                old: "b".to_owned(),
                order: 1,
            },
            changed(2, "C", "c", 2),
        ]
    );
}

#[test]
fn removes_lines_left_unpaired_at_the_end_of_a_hunk() {
    let edit = edit_of(vec![hunk(1, vec![same("a"), removed("b")])]);

    assert_eq!(
        edit.lines,
        [
            same_at(1, "a"),
            Shown::Removed {
                old: "b".to_owned(),
                order: 0,
            },
        ]
    );
}

#[test]
fn folds_unchanged_lines_beyond_two_around_each_change() {
    let mut lines: Vec<ChangeLine> = (1..=5).map(|line| same(&line.to_string())).collect();
    lines.push(added("x"));
    lines.extend((7..=12).map(|line| same(&line.to_string())));
    lines.push(added("y"));
    lines.extend((14..=17).map(|line| same(&line.to_string())));

    let edit = edit_of(vec![hunk(1, lines)]);

    let kinds: Vec<String> = edit
        .lines
        .iter()
        .map(|shown| match shown {
            Shown::Same { number, .. } => number.to_string(),
            Shown::Fold { fold, lines } => format!("fold{fold}:{}", lines.len()),
            Shown::Added { text, .. } => text.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "fold0:3", "4", "5", "x", "7", "8", "fold1:2", "11", "12", "y", "14", "15", "fold2:2"
        ]
    );
    assert_eq!(edit.folds, 3);
    assert_eq!(edit.width, 2);
}

#[test]
fn shows_short_runs_of_unchanged_lines_whole() {
    let edit = edit_of(vec![hunk(
        1,
        vec![
            same("1"),
            same("2"),
            same("3"),
            added("x"),
            same("5"),
            same("6"),
            same("7"),
            same("8"),
            same("9"),
            added("y"),
            same("11"),
            same("12"),
            same("13"),
        ],
    )]);

    assert_eq!(edit.folds, 0);
    assert_eq!(edit.lines.len(), 13);
}

#[test]
fn leaves_a_gap_between_hunks_apart() {
    let edit = edit_of(vec![
        hunk(1, vec![same("a"), added("b")]),
        hunk(10, vec![added("c")]),
        hunk(11, vec![added("d")]),
    ]);

    assert_eq!(edit.lines[2], Shown::Gap { count: 7 });
    assert_eq!(edit.lines.len(), 5);
}

#[test]
fn keeps_four_hundred_lines_and_counts_the_rest_in_a_gap() {
    let first = (0..450).map(|line| added(&line.to_string())).collect();
    let second = (0..10).map(|line| added(&line.to_string())).collect();

    let edit = edit_of(vec![hunk(1, first), hunk(500, second)]);

    assert_eq!(edit.lines.len(), KEPT_LINES + 1);
    assert_eq!(edit.lines.last(), Some(&Shown::Gap { count: 60 }));
    assert_eq!(edit.width, 3);
}

#[test]
fn keeps_room_across_hunks() {
    let first = (0..399).map(|line| added(&line.to_string())).collect();
    let second = vec![added("a"), added("b"), added("c")];

    let edit = edit_of(vec![hunk(1, first), hunk(400, second)]);

    assert_eq!(
        edit.lines[399],
        Shown::Added {
            number: 400,
            text: "a".to_owned(),
            order: 399,
        }
    );
    assert_eq!(edit.lines.last(), Some(&Shown::Gap { count: 2 }));
}

#[test]
fn counts_the_digits_of_the_largest_number() {
    assert_eq!(digits(0), 1);
    assert_eq!(digits(9), 1);
    assert_eq!(digits(10), 2);
    assert_eq!(digits(999), 3);
    assert_eq!(digits(1_000), 4);
}
