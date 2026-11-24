use std::time::Instant;

use super::*;
use crate::code::Change;
use crate::code::tests::{added, after, edited, removed, same, sight, still};

fn tally() -> Tally {
    Tally::of(&[ChangeHunk {
        new_start: 1,
        lines: vec![same("a"), removed("b"), added("B"), added("C")],
    }])
}

fn text_of(line: &Line) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn counts_the_lines_a_change_added_and_removed() {
    assert_eq!(tally().text(false), "+2 −1");
    assert_eq!(tally().text(true), "+2");
    assert_eq!(Tally::of(&[]).text(false), "+0 −0");
}

#[test]
fn heads_a_stacked_block_with_its_name_arrow_and_tally_aligned_on_the_widest_name() {
    let header = |name, created| {
        Header {
            name,
            created,
            tally: tally(),
            pad: name_columns("layout.rs", false),
            room: 96,
        }
        .line()
    };

    let widest = header("layout.rs", false);
    let shorter = header("flow.rs", false);
    let created = header("new.rs", true);

    assert_eq!(text_of(&widest), "layout.rs ↗   ··· +2 −1");
    assert_eq!(text_of(&shorter), "flow.rs ↗     ··· +2 −1");
    assert_eq!(text_of(&created), "✚ new.rs ↗    ··· +2");
    assert_eq!(widest.indent, TEXT_INDENT);
    let inks: Vec<Ink> = widest.spans.iter().map(|span| span.ink).collect();
    assert_eq!(inks, [Ink::Secondary, Ink::Secondary, Ink::Receding]);
}

#[test]
fn cuts_the_name_and_keeps_the_tally_whole_when_the_column_is_narrow() {
    let line = Header {
        name: "a_rather_long_file_name.rs",
        created: false,
        tally: tally(),
        pad: 0,
        room: 24,
    }
    .line();

    let text = text_of(&line);
    assert!(text.ends_with(" ↗   ··· +2 −1"), "{text}");
    assert!(text.contains('…'), "{text}");
    assert_eq!(columns(&text) + usize::from(TEXT_INDENT), 24);
}

#[test]
fn cuts_the_name_of_a_created_file_to_the_column_too() {
    let line = Header {
        name: "a_rather_long_file_name.rs",
        created: true,
        tally: tally(),
        pad: 0,
        room: 24,
    }
    .line();

    let text = text_of(&line);
    assert!(text.starts_with("✚ "), "{text}");
    assert_eq!(columns(&text) + usize::from(TEXT_INDENT), 24);
}

#[test]
fn ends_the_name_after_its_arrow() {
    assert_eq!(name_end("layout.rs", false), 13);
    assert_eq!(name_end("new.rs", true), 12);
    assert_eq!(name_columns("", false), 2);
}

fn block(change: Change, now: Instant) -> CodeBlock {
    crate::code::tests::grown(change, now)
}

fn created(path: &str, lines: usize) -> Change {
    Change {
        path: path.to_owned(),
        created: true,
        hunks: vec![ChangeHunk {
            new_start: 1,
            lines: (1..=lines)
                .map(|line| added(&format!("line {line}")))
                .collect(),
        }],
        neighbours: None,
    }
}

#[test]
fn folds_a_stacked_block_to_its_header_and_opens_it_from_past_its_name() {
    let start = Instant::now();
    let mut stacked = block(edited(4, vec![same("a"), removed("b"), added("B")]), start);
    let alone = stacked.row_count();

    stacked.stack(name_columns("layout.rs", false));
    let folded = stacked.row_count();
    let header = still(&stacked, 0, sight(start, false));
    let past_the_header = still(&stacked, 1, sight(start, false));
    let on_the_name = stacked.toggle(0, name_end("a.rs", false) - 1);
    let past_the_name = stacked.toggle(0, name_end("a.rs", false));

    assert_eq!(folded, 1);
    assert_eq!(header, ["  a.rs ↗        ··· +1 −1"]);
    assert_eq!(past_the_header, Vec::<String>::new());
    assert!(!on_the_name);
    assert!(past_the_name);
    assert_eq!(stacked.row_count(), alone);
    assert_eq!(still(&stacked, 0, sight(start, false)), header);
    assert!(stacked.toggle(0, 40));
    assert_eq!(stacked.row_count(), 1);
}

#[test]
fn keeps_the_rows_of_an_open_stacked_edit_under_its_header() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut plain = block(edited(4, vec![same("a"), removed("b"), added("B")]), start);
    let mut stacked = plain.clone();
    stacked.stack(0);
    stacked.toggle(0, 40);

    for row in 1..plain.row_count() {
        assert_eq!(
            still(&stacked, row, sight(later, false)),
            still(&plain, row, sight(later, false))
        );
    }
    assert!(plain.opens_file(0, 40, later));
    assert!(!stacked.opens_file(0, 40, later));
    assert!(stacked.opens_file(0, 3, later));
    assert!(!plain.toggle(0, 40));
}

#[test]
fn puts_an_open_created_file_under_its_header_and_opens_it_once_written() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let plain = block(created("src/new.rs", 3), start);
    let mut stacked = plain.clone();
    stacked.stack(0);

    let folded_header = still(&stacked, 0, sight(later, false));
    let written_yet = stacked.opens_file(0, 3, start);
    stacked.toggle(0, 40);

    assert_eq!(folded_header, ["  ✚ new.rs ↗   ··· +3"]);
    assert!(!written_yet);
    assert!(stacked.opens_file(0, 3, later));
    assert_eq!(stacked.row_count(), plain.row_count() + 1);
    for row in 0..plain.row_count() {
        assert_eq!(
            still(&stacked, row + 1, sight(later, false)),
            still(&plain, row, sight(later, false))
        );
    }
    assert!(stacked.opens_file(2, 0, later));
    assert!(!stacked.opens_file(1, 0, later));
}

#[test]
fn keeps_whether_a_stacked_block_is_open_when_the_stack_grows() {
    let start = Instant::now();
    let mut stacked = block(edited(4, vec![removed("b"), added("B")]), start);
    stacked.stack(0);
    stacked.toggle(0, 40);
    let open = stacked.row_count();

    stacked.stack(30);

    assert_eq!(stacked.row_count(), open);
    let header = &still(&stacked, 0, sight(start, false))[0];
    assert_eq!(
        header.chars().position(|character| character == '·'),
        Some(35)
    );
}
