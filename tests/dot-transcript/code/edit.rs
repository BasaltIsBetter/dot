use std::time::Instant;

use super::*;
use crate::code::tests::{added, after, drawn, edited, removed, same, sight, still, text_of};
use crate::code::{ChangeLine, CodeBlock};
use crate::ink::opacity;

fn block(lines: Vec<ChangeLine>, start: Instant) -> CodeBlock {
    CodeBlock::new(edited(10, lines), start).unwrap()
}

fn rust_edit(start: Instant) -> CodeBlock {
    block(
        vec![
            same("fn a() {"),
            removed("    let x = 1;"),
            added("    let x = 2;"),
            added("    let y = 3;"),
            same("}"),
        ],
        start,
    )
}

fn still_line(block: &CodeBlock, row: usize, sight: crate::code::Sight<'_>) -> Line {
    match drawn(block, row, sight).into_iter().next() {
        Some(Drawn::Still(line)) => line,
        other => panic!("expected a still line, got {other:?}"),
    }
}

#[test]
fn shows_each_line_after_the_one_before() {
    let start = Instant::now();
    let block = rust_edit(start);
    let at = sight(after(start, 100), false);

    assert_eq!(still(&block, 1, at), ["   10 │ fn a() {"]);
    assert_eq!(still(&block, 2, at), ["   11 │     let x = 1;"]);
    assert_eq!(still(&block, 3, at), Vec::<String>::new());
    assert_eq!(still(&block, 5, at), Vec::<String>::new());
}

#[test]
fn changes_only_the_words_that_differ_one_line_after_the_other() {
    let start = Instant::now();
    let block = rust_edit(start);

    let changing = drawn(&block, 2, sight(after(start, 430), false));

    assert_eq!(
        still(&block, 2, sight(after(start, 430), false)),
        [
            "   11 │     let x =  ;".to_owned(),
            format!("rising:{}2", " ".repeat(20)),
            format!("leaving:{}1", " ".repeat(20)),
        ]
    );
    let Drawn::Rising(rising, eased) = &changing[1] else {
        panic!("the new word rises");
    };
    assert!(rising.spans[0].heat > 0 && rising.spans[0].heat < u8::MAX);
    assert!(*eased > 0.0 && *eased < 1.0);
    let Drawn::Leaving(_, leaving) = &changing[2] else {
        panic!("the old word leaves");
    };
    assert!((eased - leaving).abs() < f64::EPSILON);
    assert_eq!(
        still(&block, 3, sight(after(start, 430), false)),
        ["   12 │ "]
    );
}

#[test]
fn starts_each_change_at_the_very_start_of_its_turn() {
    let start = Instant::now();
    let edit = rust_edit(start);
    let removal = block(vec![same("a"), removed("b"), same("c")], start);

    assert_eq!(
        still(&edit, 2, sight(after(start, 279), false)),
        ["   11 │     let x = 1;"]
    );
    assert_eq!(
        still(&edit, 2, sight(after(start, 280), false)),
        [
            "   11 │     let x =  ;".to_owned(),
            format!("rising:{}2", " ".repeat(20)),
            format!("leaving:{}1", " ".repeat(20)),
        ]
    );
    assert_eq!(
        still(&removal, 2, sight(after(start, 209), false)),
        ["      │ b"]
    );
    assert_eq!(
        still(&removal, 2, sight(after(start, 210), false)),
        ["      │ ".to_owned(), format!("leaving:{}b", " ".repeat(8))]
    );
}

#[test]
fn ends_a_change_on_its_new_text_in_the_gutter_ink() {
    let start = Instant::now();
    let block = rust_edit(start);

    let line = still_line(&block, 2, sight(after(start, 600), false));

    assert_eq!(text_of(&line), "   11 │     let x = 2;");
    assert_eq!(line.spans[0].ink, Ink::Receding);
    assert_eq!(line.alpha, crate::ink::OPAQUE);
}

#[test]
fn marks_each_change_once_settled_and_recedes_the_context() {
    let start = Instant::now();
    let block = rust_edit(start);
    let settled = sight(after(start, 1_330), false);
    let before = sight(after(start, 1_329), false);

    let changed = still_line(&block, 2, settled);
    let added_line = still_line(&block, 3, settled);
    let context = still_line(&block, 1, settled);

    assert_eq!(text_of(&changed), "── 11 │     let x = 2;");
    assert_eq!(changed.spans[0].ink, Ink::Full);
    assert_eq!(text_of(&added_line), "─  12 │     let y = 3;");
    assert_eq!(added_line.spans[0].ink, Ink::Full);
    assert_eq!(context.alpha, opacity(0.4));
    assert_eq!(still_line(&block, 1, before).alpha, crate::ink::OPAQUE);
    assert_eq!(
        text_of(&still_line(&block, 2, before)),
        "   11 │     let x = 2;"
    );
}

#[test]
fn shows_the_old_line_under_the_pointer_once_settled() {
    let start = Instant::now();
    let block = rust_edit(start);

    let hovered = still_line(&block, 2, sight(after(start, 1_400), true));
    let early = still_line(&block, 2, sight(after(start, 600), true));

    assert_eq!(text_of(&hovered), "── 11 │     let x = 1;");
    assert!(
        hovered.spans[1..]
            .iter()
            .all(|span| span.ink == Ink::Receding)
    );
    assert_eq!(text_of(&early), "   11 │     let x = 2;");
}

#[test]
fn raises_an_added_line_whole() {
    let start = Instant::now();
    let block = rust_edit(start);

    assert_eq!(
        still(&block, 3, sight(after(start, 729), false)),
        ["   12 │ "]
    );
    assert_eq!(
        still(&block, 3, sight(after(start, 800), false)),
        [
            "   12 │ ".to_owned(),
            format!("rising:{}    let y = 3;", " ".repeat(8))
        ]
    );
    assert_eq!(
        still(&block, 3, sight(after(start, 1_030), false)),
        ["   12 │     let y = 3;"]
    );
}

#[test]
fn lets_a_removed_line_leave_and_keeps_its_row() {
    let start = Instant::now();
    let block = block(vec![same("a"), removed("b"), same("c")], start);

    assert_eq!(
        still(&block, 2, sight(after(start, 100), false)),
        ["      │ b"]
    );
    assert_eq!(
        still(&block, 2, sight(after(start, 300), false)),
        ["      │ ".to_owned(), format!("leaving:{}b", " ".repeat(8))]
    );
    assert_eq!(
        still(&block, 2, sight(after(start, 600), false)),
        ["      │ "]
    );
    let settled = still_line(&block, 2, sight(after(start, 900), false));
    assert_eq!(text_of(&settled), "──    │ ");
    assert_eq!(settled.spans[0].ink, Ink::Full);
    let hovered = still_line(&block, 2, sight(after(start, 900), true));
    assert_eq!(text_of(&hovered), "──    │ b");
    assert_eq!(hovered.spans[1].ink, Ink::Receding);
}

#[test]
fn folds_far_context_and_shows_it_receded_once_open() {
    let start = Instant::now();
    let mut lines: Vec<ChangeLine> = (1..=8).map(|line| same(&format!("line {line}"))).collect();
    lines.push(added("new"));
    let mut block = block(lines, start);
    let later = sight(after(start, 5_000), false);

    let closed = still_line(&block, 1, later);
    assert_eq!(text_of(&closed), "   ·· │ ··· 6 lines");
    assert_eq!(closed.spans[0].ink, Ink::Receding);

    assert!(block.toggle(1));
    let first = still_line(&block, 1, sight(start, false));
    assert_eq!(text_of(&first), "   10 │ line 1");
    assert_eq!(first.alpha, opacity(0.4));
    assert_eq!(still_line(&block, 1, later).alpha, opacity(0.4));
    assert_eq!(text_of(&still_line(&block, 6, later)), "   15 │ line 6");
}

#[test]
fn counts_lines_it_does_not_have_in_the_faintest_ink() {
    let start = Instant::now();
    let change = crate::code::Change {
        hunks: vec![
            crate::code::ChangeHunk {
                new_start: 1,
                lines: vec![added("a")],
            },
            crate::code::ChangeHunk {
                new_start: 9,
                lines: vec![added("b")],
            },
        ],
        ..edited(1, Vec::new())
    };
    let block = CodeBlock::new(change, start).unwrap();

    let gap = still_line(&block, 2, sight(after(start, 5_000), false));

    assert_eq!(text_of(&gap), "   · │ ··· 7 lines");
    assert!(gap.spans.iter().all(|span| span.ink == Ink::Faint));
}

#[test]
fn cuts_a_long_line_at_the_edge_of_the_column() {
    let start = Instant::now();
    let block = block(vec![same("abcdefghij")], start);
    let narrow = crate::code::Sight {
        room: 12,
        ..sight(start, false)
    };

    assert_eq!(text_of(&still_line(&block, 1, narrow)), "   10 │ abc…");
}

#[test]
fn finds_the_words_that_changed_between_two_lines() {
    assert_eq!(middles("a b", "a b"), (3..3, 3..3));
    assert_eq!(middles("x", "y"), (0..1, 0..1));
    assert_eq!(middles("f(a)", "f(a, b)"), (3..3, 3..6));
    assert_eq!(middles("", "new"), (0..0, 0..3));
}

#[test]
fn tells_the_largest_number_of_each_kind_of_line() {
    let fold = Shown::Fold {
        fold: 0,
        lines: vec![(3, String::new()), (4, String::new())],
    };

    assert_eq!(fold.last_number(), 4);
    assert_eq!(
        Shown::Fold {
            fold: 0,
            lines: Vec::new()
        }
        .last_number(),
        0
    );
    assert_eq!(Shown::Gap { count: 9 }.last_number(), 0);
    assert_eq!(
        Shown::Removed {
            old: String::new(),
            order: 0
        }
        .last_number(),
        0
    );
}
