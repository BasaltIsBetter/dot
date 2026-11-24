use std::borrow::Cow;

use super::*;

pub(crate) struct Words;

impl CodeWords for Words {
    fn lines(&self, count: u64) -> Cow<'_, str> {
        Cow::Borrowed(if count == 1 { "line" } else { "lines" })
    }

    fn new_file(&self) -> Cow<'_, str> {
        Cow::Borrowed("new")
    }

    fn fold_back(&self) -> Cow<'_, str> {
        Cow::Borrowed("fold")
    }
}

pub(crate) fn same(text: &str) -> ChangeLine {
    ChangeLine::Same(text.to_owned())
}

pub(crate) fn removed(text: &str) -> ChangeLine {
    ChangeLine::Removed(text.to_owned())
}

pub(crate) fn added(text: &str) -> ChangeLine {
    ChangeLine::Added(text.to_owned())
}

pub(crate) fn edited(new_start: u64, lines: Vec<ChangeLine>) -> Change {
    Change {
        path: "src/a.rs".to_owned(),
        created: false,
        hunks: vec![ChangeHunk { new_start, lines }],
        neighbours: None,
    }
}

pub(crate) const GROWN: u64 = 10_000;

pub(crate) fn grown(change: Change, start: Instant) -> CodeBlock {
    let mut block = CodeBlock::new(change, start).unwrap();
    block.grow(after(start, GROWN));
    block
}

pub(crate) fn sight(now: Instant, hovered: bool) -> Sight<'static> {
    Sight {
        now,
        hovered,
        room: 96,
        words: &Words,
    }
}

pub(crate) fn drawn(block: &CodeBlock, row: usize, sight: Sight<'_>) -> Vec<Drawn> {
    let mut drawn = Vec::new();
    block.draw(row, sight, &mut |piece| drawn.push(piece));
    drawn
}

pub(crate) fn text_of(line: &Line) -> String {
    let spans: String = line.spans.iter().map(|span| span.text.as_str()).collect();
    format!("{}{spans}", " ".repeat(usize::from(line.indent)))
}

pub(crate) fn still(block: &CodeBlock, row: usize, sight: Sight<'_>) -> Vec<String> {
    drawn(block, row, sight)
        .iter()
        .map(|piece| match piece {
            Drawn::Still(line) => text_of(line),
            Drawn::Rising(line, _) => format!("rising:{}", text_of(line)),
            Drawn::Leaving(line, _) => format!("leaving:{}", text_of(line)),
        })
        .collect()
}

pub(crate) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn folding() -> Change {
    let mut lines: Vec<ChangeLine> = (1..=8)
        .map(|line| same(&format!("before {line}")))
        .collect();
    lines.push(removed("old"));
    lines.push(added("new"));
    lines.extend((1..=8).map(|line| same(&format!("after {line}"))));
    edited(1, lines)
}

#[test]
fn has_nothing_to_show_without_a_line() {
    let start = Instant::now();
    let empty = Change {
        path: "a.rs".to_owned(),
        ..Change::default()
    };
    let created = Change {
        created: true,
        ..empty.clone()
    };

    assert_eq!(CodeBlock::new(empty, start), None);
    assert_eq!(CodeBlock::new(created, start), None);
}

#[test]
fn counts_the_name_row_then_one_row_per_shown_line() {
    let block = grown(folding(), Instant::now());

    assert_eq!(block.row_count(), 1 + 1 + 2 + 1 + 2 + 1);
    assert_eq!(block.path(), "src/a.rs");
}

#[test]
fn opens_and_closes_folded_lines_where_they_are_clicked() {
    let start = Instant::now();
    let mut block = grown(folding(), start);

    assert!(!block.toggle(2, 0));
    assert!(block.toggle(1, 0));
    assert_eq!(block.row_count(), 1 + 6 + 2 + 1 + 2 + 1);
    assert!(block.toggle(4, 0));
    assert_eq!(block.row_count(), 1 + 1 + 2 + 1 + 2 + 1);
    assert!(block.toggle(7, 0));
    assert_eq!(block.row_count(), 1 + 1 + 2 + 1 + 2 + 6);
}

#[test]
fn opens_an_edited_file_from_its_name_row_only() {
    let start = Instant::now();
    let block = grown(folding(), start);

    assert!(block.opens_file(0, 0, start));
    assert!(!block.opens_file(1, 0, start));
}

#[test]
fn opens_and_marks_a_file_outside_the_folder_too() {
    let start = Instant::now();
    let edited = Change {
        path: "/Users/someone/Desktop/notes.md".to_owned(),
        ..folding()
    };
    let created = Change {
        path: "/Users/someone/Desktop/tool/run.py".to_owned(),
        created: true,
        hunks: vec![ChangeHunk {
            new_start: 1,
            lines: vec![added("print(1)")],
        }],
        neighbours: None,
    };
    let edited = grown(edited, start);
    let created = grown(created, start);
    let written = after(start, 2_000);

    assert!(edited.opens_file(0, 0, start));
    assert_eq!(still(&edited, 0, sight(start, false)), ["  notes.md ↗"]);
    assert_eq!(still(&created, 0, sight(written, false)), ["  tool/"]);
    assert!(created.opens_file(1, 0, written));
    assert_eq!(
        still(&created, 1, sight(written, false)),
        ["  ✚ run.py ↗ new · 1 line"]
    );
}

#[test]
fn names_the_file_without_its_folders_and_marks_it_as_opening() {
    let start = Instant::now();
    let block = grown(folding(), start);

    let name = drawn(&block, 0, sight(start, false));

    assert_eq!(name.len(), 1);
    let Drawn::Still(line) = &name[0] else {
        panic!("the name stays still");
    };
    assert_eq!(text_of(line), "  a.rs ↗");
    assert!(line.spans.iter().all(|span| span.ink == Ink::Secondary));
}

#[test]
fn lasts_until_its_last_change_settles() {
    let start = Instant::now();
    let block = grown(folding(), start);

    assert_eq!(block.lasts_until(), after(start, 70 * 7 + 600));
}

#[test]
fn shows_a_created_file_from_its_folder() {
    let start = Instant::now();
    let created = Change {
        path: "src/new.rs".to_owned(),
        created: true,
        hunks: vec![ChangeHunk {
            new_start: 1,
            lines: vec![added("one"), added("two")],
        }],
        neighbours: None,
    };
    let mut block = grown(created, start);

    assert_eq!(still(&block, 0, sight(start, false)), ["  src/"]);
    assert_eq!(block.row_count(), 1 + 1 + 1 + 2);
    assert!(!block.toggle(4, 0));
    assert!(block.opens_file(1, 0, after(start, 2_000)));
}

#[test]
fn ends_a_label_with_its_count_and_unit() {
    let line = folded(2, 6, Ink::Receding, &Words);

    assert_eq!(text_of(&line), "   ·· │ ··· 6 lines");
    assert!(line.spans.iter().all(|span| span.ink == Ink::Receding));
    assert_eq!(
        text_of(&folded(1, 6, Ink::Receding, &Words)),
        "   · │ ··· 6 lines"
    );
    assert_eq!(
        text_of(&folded(3, 6, Ink::Receding, &Words)),
        "    ·· │ ··· 6 lines"
    );
}

#[test]
fn slices_the_inks_of_a_line_without_losing_a_byte() {
    let spans = sliced("let x = 1;", CodeLanguage::Rust, 2..9);

    let text: String = spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(text, "t x = 1");
    assert_eq!(spans[0].ink, Ink::Secondary);
    assert_eq!(sliced("let", CodeLanguage::Rust, 3..3), Vec::<Span>::new());
}

#[test]
fn paces_lines_so_that_many_still_take_little_time() {
    let every = Duration::from_millis(70);
    let within = Duration::from_millis(1_000);

    assert_eq!(pace(every, within, 0), every);
    assert_eq!(pace(every, within, 10), every);
    assert_eq!(pace(every, within, 100), Duration::from_millis(10));
    assert_eq!(count32(usize::MAX), u32::MAX);
}

#[test]
fn hands_the_inks_of_a_line_to_whoever_draws_code() {
    assert_eq!(
        code_inks("fn x", CodeLanguage::of("a.rs")),
        [(0..3, Ink::Secondary), (3..4, Ink::Full)]
    );
}
