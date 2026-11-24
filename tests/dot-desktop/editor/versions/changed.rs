use std::fmt::Write as _;

use proptest::prelude::*;

use super::*;

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
        Some(vec![true, true, false])
    );
    assert_eq!(
        common(&["b", "c", "a"], &["a", "b", "c"]),
        Some(vec![false, true, true])
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

        let kept = common(&old, &new).unwrap();

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

fn longest_shared_length(old: &[&str], new: &[&str]) -> usize {
    let mut longest = vec![vec![0_usize; new.len() + 1]; old.len() + 1];
    for row in (0..old.len()).rev() {
        for column in (0..new.len()).rev() {
            longest[row][column] = if old[row] == new[column] {
                longest[row + 1][column + 1] + 1
            } else {
                longest[row + 1][column].max(longest[row][column + 1])
            };
        }
    }
    longest[0][0]
}

proptest! {
    #[test]
    fn keeps_as_many_lines_as_a_full_comparison_on_longer_texts(
        old in proptest::collection::vec("[abcd]", 0..60),
        new in proptest::collection::vec("[abcd]", 0..60),
    ) {
        let old: Vec<&str> = old.iter().map(String::as_str).collect();
        let new: Vec<&str> = new.iter().map(String::as_str).collect();

        let kept = common(&old, &new).unwrap();

        let shared: Vec<&str> = new
            .iter()
            .zip(&kept)
            .filter(|&(_, &kept)| kept)
            .map(|(line, _)| *line)
            .collect();
        prop_assert!(in_order(&shared, &old));
        prop_assert_eq!(shared.len(), longest_shared_length(&old, &new));
    }
}

#[test]
fn compares_a_long_file_with_a_few_changes_line_by_line() {
    let original: Vec<String> = (0..20_000).map(|line| format!("line {line}")).collect();
    let mut edited = original.clone();
    edited[3] = "changed".to_owned();
    edited.remove(10_000);
    edited.insert(19_000, "added".to_owned());

    let lines = changed_lines(&original.join("\n"), &edited.join("\n"));

    assert_eq!(lines.changed.iter().filter(|&&changed| changed).count(), 2);
    assert_eq!(lines.removed, 2);
}

#[test]
fn gives_up_on_a_middle_with_more_changes_than_it_compares() {
    let old: Vec<String> = (0..1_500).map(|line| format!("old {line}")).collect();
    let new: Vec<String> = (0..1_500).map(|line| format!("new {line}")).collect();
    let old: Vec<&str> = old.iter().map(String::as_str).collect();
    let new: Vec<&str> = new.iter().map(String::as_str).collect();

    assert_eq!(common(&old, &new), None);
    assert!(common(&old[..MOST_EDITS / 2], &new[..MOST_EDITS / 2]).is_some());
    assert_eq!(
        common(&old[..=MOST_EDITS / 2], &new[..=MOST_EDITS / 2]),
        None
    );
}

#[test]
fn gives_up_once_it_compared_more_lines_than_it_may() {
    let marked = |mark: &'static str| -> Vec<&str> {
        (0..3_000)
            .map(|line| if line % 10 == 0 { mark } else { "" })
            .collect()
    };

    let (old, new) = (marked("a"), marked("b"));

    assert!(frontiers(&old, &new, MOST_COMPARISONS).is_some());
    assert_eq!(frontiers(&old, &new, 10_000), None);
    assert!(frontiers(&old[..100], &new[..100], 10_000).is_some());
}
