use std::time::Instant;

use super::*;
use crate::code::tests::{added, after, drawn, grown, sight, still, text_of};
use crate::code::{Change, CodeBlock};

fn created(lines: usize, neighbours: Option<Neighbours>) -> Change {
    Change {
        path: "src/new.rs".to_owned(),
        created: true,
        hunks: vec![ChangeHunk {
            new_start: 1,
            lines: (1..=lines).map(|line| added(&format!("l{line}"))).collect(),
        }],
        neighbours,
    }
}

fn around() -> Neighbours {
    Neighbours {
        folder: "src/".to_owned(),
        before: vec!["lib.rs".to_owned()],
        after: vec!["old.rs".to_owned(), "z.rs".to_owned()],
    }
}

fn still_line(block: &CodeBlock, row: usize, at: crate::code::Sight<'_>) -> Line {
    match drawn(block, row, at).into_iter().next() {
        Some(Drawn::Still(line)) => line,
        other => panic!("expected a still line, got {other:?}"),
    }
}

#[test]
fn places_the_file_among_its_neighbours() {
    let start = Instant::now();
    let block = grown(created(12, Some(around())), start);
    let at = sight(start, false);

    assert_eq!(still(&block, 0, at), ["  src/"]);
    assert_eq!(still(&block, 1, at), ["    lib.rs"]);
    assert_eq!(still(&block, 2, at), ["  ✚ new.rs   new · 0 lines"]);
    assert_eq!(still(&block, 3, at), ["    old.rs"]);
    assert_eq!(still(&block, 4, at), ["    z.rs"]);
    assert_eq!(still(&block, 5, at), Vec::<String>::new());
    assert_eq!(still_line(&block, 1, at).spans[0].ink, Ink::Receding);
    assert_eq!(block.row_count(), 6 + 10 + 1);
}

#[test]
fn writes_line_after_line_with_a_counter_in_orange() {
    let start = Instant::now();
    let block = grown(created(12, Some(around())), start);

    let writing = still_line(&block, 2, sight(after(start, 1_100), false));
    let written = still_line(&block, 2, sight(after(start, 1_180), false));

    assert_eq!(text_of(&writing), "  ✚ new.rs   new · 11 lines");
    assert_eq!(writing.spans[2].heat, OPAQUE);
    assert_eq!(text_of(&written), "  ✚ new.rs ↗ new · 12 lines");
    assert_eq!(written.spans[2].ink, Ink::Secondary);
    assert_eq!(written.spans[2].heat, 0);
    assert!(!block.opens_file(2, 0, after(start, 1_100)));
    assert!(block.opens_file(2, 0, after(start, 1_180)));
    assert!(!block.opens_file(1, 0, after(start, 1_180)));
}

#[test]
fn raises_each_line_from_below_as_it_is_written() {
    let start = Instant::now();
    let block = grown(created(12, Some(around())), start);

    assert_eq!(
        still(&block, 6, sight(after(start, 299), false)),
        Vec::<String>::new()
    );
    assert_eq!(
        still(&block, 6, sight(after(start, 400), false)),
        ["    1 │ ".to_owned(), format!("rising:{}l1", " ".repeat(8))]
    );
    assert_eq!(
        still(&block, 6, sight(after(start, 600), false)),
        ["    1 │ l1"]
    );
    let Some(Drawn::Rising(line, _)) = drawn(&block, 7, sight(after(start, 400), false)).pop()
    else {
        panic!("the second line rises");
    };
    assert!(line.spans[0].heat > 0);
}

#[test]
fn folds_what_comes_after_ten_lines_and_counts_it_as_it_climbs() {
    let start = Instant::now();
    let block = grown(created(12, Some(around())), start);

    assert_eq!(
        still(&block, 16, sight(after(start, 1_000), false)),
        Vec::<String>::new()
    );
    let climbing = still_line(&block, 16, sight(after(start, 1_100), false));
    let done = still_line(&block, 16, sight(after(start, 1_200), false));

    assert_eq!(text_of(&climbing), "   ·· │ ··· 1 line");
    assert!(climbing.spans.iter().all(|span| span.heat == OPAQUE));
    assert_eq!(text_of(&done), "   ·· │ ··· 2 lines");
    assert!(
        done.spans
            .iter()
            .all(|span| span.ink == Ink::Receding && span.heat == 0)
    );
}

#[test]
fn unfolds_every_line_then_folds_back_from_the_last_row() {
    let start = Instant::now();
    let mut block = grown(created(12, Some(around())), start);
    let later = sight(after(start, 5_000), false);

    assert!(!block.toggle(15, 0));
    assert!(block.toggle(16, 0));
    assert_eq!(block.row_count(), 6 + 12 + 1);
    assert_eq!(still(&block, 17, later), ["   12 │ l12"]);
    assert_eq!(still(&block, 18, later), ["   ·· │ ··· fold"]);
    assert!(!block.toggle(17, 0));
    assert!(block.toggle(18, 0));
    assert_eq!(block.row_count(), 6 + 10 + 1);
}

#[test]
fn needs_no_fold_for_ten_lines_or_fewer() {
    let start = Instant::now();
    let mut block = grown(created(10, None), start);

    assert_eq!(block.row_count(), 3 + 10);
    assert!(!block.toggle(13, 0));
    assert_eq!(still(&block, 0, sight(start, false)), ["  src/"]);
}

#[test]
fn counts_lines_beyond_four_hundred_without_showing_them() {
    let start = Instant::now();
    let mut block = grown(created(405, None), start);
    let later = sight(after(start, 5_000), false);

    assert_eq!(still(&block, 1, later), ["  ✚ new.rs ↗ new · 405 lines"]);
    assert_eq!(still(&block, 13, later), ["    ·· │ ··· 395 lines"]);
    assert!(block.toggle(13, 0));
    assert_eq!(block.row_count(), 3 + 400 + 2);
    let gap = still_line(&block, 403, later);
    assert_eq!(text_of(&gap), "    ·· │ ··· 5 lines");
    assert!(gap.spans.iter().all(|span| span.ink == Ink::Faint));
    assert_eq!(still(&block, 404, later), ["    ·· │ ··· fold"]);
    assert!(block.toggle(404, 0));
}

#[test]
fn writes_a_long_file_within_two_seconds() {
    let start = Instant::now();
    let block = grown(created(400, None), start);

    assert_eq!(block.lasts_until(), after(start, 300 + 2_000 + 600));
}

#[test]
fn names_the_folder_of_a_file_without_neighbours() {
    assert_eq!(folder_of("crates/a/src/new.rs"), "src/");
    assert_eq!(folder_of("new.rs"), "./");
    assert_eq!(folder_of("/new.rs"), "./");
}

#[test]
fn keeps_the_first_lines_of_a_huge_created_file_and_counts_the_rest() {
    let change = created(300_000, None);

    let created = Created::of(&change.path, change.hunks, change.neighbours);

    assert_eq!(created.lines.len(), KEPT_LINES);
    assert_eq!(created.beyond, 300_000 - 400);
}

#[test]
fn keeps_each_row_above_a_created_file_within_the_column() {
    let start = Instant::now();
    let long = |letter: &str| letter.repeat(200);
    let mut change = created(
        3,
        Some(Neighbours {
            folder: format!("{}/", long("f")),
            before: vec![long("b")],
            after: vec![long("a")],
        }),
    );
    change.path = format!("{}/{}.rs", long("f"), long("n"));
    let block = grown(change, start);
    let settled = after(start, 60_000);

    for row in 0..4 {
        let line = still_line(&block, row, sight(settled, false));
        assert!(
            crate::line::columns(&text_of(&line)) <= 96,
            "{row}: {}",
            text_of(&line)
        );
        assert!(text_of(&line).contains('…'), "{row}: {}", text_of(&line));
    }
}

#[test]
fn grows_from_its_folder_a_row_per_line_written_then_its_fold() {
    let start = Instant::now();
    let mut block = CodeBlock::new(created(12, Some(around())), start).unwrap();

    assert_eq!(block.row_count(), 6);
    block.grow(after(start, 300));
    assert_eq!(block.row_count(), 6 + 1);
    block.grow(after(start, 300 + 80 * 9));
    assert_eq!(block.row_count(), 6 + 10);
    block.grow(after(start, 300 + 80 * 10));
    assert_eq!(block.row_count(), 6 + 10 + 1);
    assert!(block.is_growing());
    block.grow(after(start, 300 + 80 * 11));
    assert!(!block.is_growing());
    assert_eq!(block.row_count(), 6 + 10 + 1);
}

#[test]
fn has_all_its_rows_once_its_last_line_is_written() {
    let start = Instant::now();
    let block = CodeBlock::new(created(12, Some(around())), after(start, 50)).unwrap();

    assert_eq!(block.grown_at(), after(start, 50 + 300 + 80 * 11));
}
