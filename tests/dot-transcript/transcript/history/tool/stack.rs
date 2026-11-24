use std::time::Instant;

use super::*;
use crate::code::tests::{sight, still};
use crate::transcript::entry::Kind;
use crate::transcript::history::tool::tests::{
    after, change_of, edit, gutter_rows, read, replying,
};

fn changed(transcript: &mut Transcript, id: &str, path: &str, now: Instant) {
    transcript.start_tool(id.to_owned(), edit(path), now);
    transcript.change_tool(id, change_of(path), now);
    transcript.grow_code(after(now, crate::code::tests::GROWN));
}

#[test]
fn folds_a_block_to_one_row_when_another_comes_right_after_it() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    changed(&mut transcript, "t1", "src/a.rs", start);
    let first = transcript.total() - 5;
    transcript.take_insertions();

    changed(&mut transcript, "t2", "src/b.rs", after(start, 10));

    assert_eq!(transcript.total(), first + 1 + 5);
    assert_eq!(transcript.file_at(first, 2, start), Some("src/a.rs"));
    assert_eq!(transcript.file_at(first + 1, 2, start), Some("src/b.rs"));
    assert!(transcript.take_insertions().contains(&Insertion {
        at: rows(first + 5),
        rows: -4.0,
    }));
    assert_eq!(
        gutter_rows(&transcript),
        [
            (first, "src/a.rs".to_owned()),
            (first + 1, "src/b.rs".to_owned())
        ]
    );
}

#[test]
fn opens_a_folded_block_from_past_its_name_and_its_file_from_its_name() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    changed(&mut transcript, "t1", "src/layout.rs", start);
    changed(&mut transcript, "t2", "src/b.rs", start);
    changed(&mut transcript, "t3", "src/c.rs", start);
    let first = transcript.total() - 7;

    let names = [
        transcript.file_at(first, 12, start).map(str::to_owned),
        transcript.file_at(first + 1, 7, start).map(str::to_owned),
        transcript.file_at(first + 1, 8, start).map(str::to_owned),
    ];
    let on_the_name = transcript.toggle_code(first + 1, 7);
    let past_the_name = transcript.toggle_code(first + 1, 12);
    let opened = transcript.total();
    transcript.toggle_code(first + 1, 12);

    assert_eq!(
        names,
        [
            Some("src/layout.rs".to_owned()),
            Some("src/b.rs".to_owned()),
            None
        ]
    );
    let dots = |position: usize| {
        let code = transcript.entries[position].code().unwrap();
        let header = &still(code, 0, sight(start, false))[0];
        header.chars().position(|character| character == '·')
    };
    let stacked: Vec<usize> = (0..transcript.entries.len())
        .filter(|&position| transcript.entries[position].code().is_some())
        .collect();
    assert_eq!(dots(stacked[0]), Some(16));
    assert_eq!(dots(stacked[1]), dots(stacked[0]));
    assert!(!on_the_name);
    assert!(past_the_name);
    assert_eq!(opened, first + 1 + 5 + 5);
    assert_eq!(transcript.total(), first + 7);
}

#[test]
fn keeps_blocks_apart_when_text_comes_between_them() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    changed(&mut transcript, "t1", "src/a.rs", start);
    let first = transcript.total() - 5;

    transcript.hear(Kind::Answer, "Now b.", start);
    changed(&mut transcript, "t2", "src/b.rs", start);

    assert_eq!(transcript.total(), first + 5 + 1 + 1 + 1 + 5);
    assert_eq!(transcript.file_at(first + 1, 30, start), None);
    assert!(transcript.toggle_code(first + 1, 30));
}

#[test]
fn stacks_blocks_across_a_tool_that_takes_no_row() {
    let start = Instant::now();
    let mut transcript = replying("fix it", start);
    changed(&mut transcript, "t1", "src/a.rs", start);
    let first = transcript.total() - 5;

    transcript.start_tool("r1".to_owned(), read("x.rs"), start);
    transcript.end_tool("r1", "12 l.".to_owned(), None, start);
    changed(&mut transcript, "t2", "src/b.rs", start);

    assert_eq!(transcript.total(), first + 1 + 5);
}
