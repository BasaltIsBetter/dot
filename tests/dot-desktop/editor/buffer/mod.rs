use super::*;
use crate::editor::tests::after;

fn buffer(text: &str) -> Buffer {
    Buffer::new(text.to_owned(), TextWidth::new(40))
}

#[test]
fn types_where_the_caret_is_and_replaces_a_selection() {
    let start = Instant::now();
    let mut buffer = buffer("let x;");
    buffer.go(Motion::RowEnd, false, start);

    assert!(buffer.insert(" // y", start));
    buffer.select_all();
    assert_eq!(buffer.selected(), Some("let x; // y"));
    assert!(buffer.insert("z", after(start, 10)));
    assert_eq!(buffer.text(), "z");
    assert_eq!(buffer.selection(), None);
    assert!(!buffer.insert("", after(start, 20)));
}

#[test]
fn selects_with_shift_and_lets_go_without_it() {
    let start = Instant::now();
    let mut buffer = buffer("abc");

    buffer.go(Motion::Right, true, start);
    buffer.go(Motion::Right, true, start);
    assert_eq!(buffer.selection(), Some(0..2));
    buffer.go(Motion::Left, false, start);
    assert_eq!(buffer.selection(), None);
    buffer.place(0, 3, true);
    assert_eq!(buffer.selected(), Some("bc"));
    buffer.place(0, 0, false);
    assert_eq!(buffer.selection(), None);
}

#[test]
fn erases_the_selection_or_toward_the_motion() {
    let start = Instant::now();
    let mut buffer = buffer("one two");
    buffer.go(Motion::RowEnd, false, start);

    assert!(buffer.erase(Motion::WordLeft, start));
    assert_eq!(buffer.text(), "one ");
    buffer.go(Motion::RowStart, false, start);
    buffer.go(Motion::Right, true, start);
    assert!(buffer.erase(Motion::Left, after(start, 5_000)));
    assert_eq!(buffer.text(), "ne ");
    assert!(!buffer.erase(Motion::Left, after(start, 5_000)));
}

#[test]
fn undoes_what_was_typed_in_one_go_then_redoes_it() {
    let start = Instant::now();
    let mut buffer = buffer("");

    for (index, letter) in ["a", "b", "c"].iter().enumerate() {
        buffer.insert(letter, after(start, 100 * index as u64));
    }
    buffer.insert("d", after(start, 2_000));

    assert!(buffer.undo(after(start, 3_000)));
    assert_eq!(buffer.text(), "abc");
    assert!(buffer.undo(after(start, 3_000)));
    assert_eq!(buffer.text(), "");
    assert!(!buffer.undo(after(start, 3_000)));
    assert!(buffer.redo(after(start, 3_000)));
    assert_eq!(buffer.text(), "abc");
    assert_eq!(buffer.caret_cell(), (0, 3));
    assert!(buffer.redo(after(start, 3_000)));
    assert_eq!(buffer.text(), "abcd");
    assert!(!buffer.redo(after(start, 3_000)));
}

#[test]
fn starts_a_new_step_at_each_line_break_and_forgets_the_redone_after_a_change() {
    let start = Instant::now();
    let mut buffer = buffer("");
    buffer.insert("a", start);
    buffer.insert("\n", after(start, 10));
    buffer.insert("b", after(start, 20));

    buffer.undo(after(start, 30));
    assert_eq!(buffer.text(), "a\n");
    buffer.insert("c", after(start, 40));
    assert!(!buffer.redo(after(start, 50)));
    buffer.undo(after(start, 60));
    buffer.undo(after(start, 60));
    assert_eq!(buffer.text(), "a");
}

#[test]
fn starts_a_new_step_after_a_pause_of_a_second() {
    let start = Instant::now();
    let mut buffer = buffer("");
    buffer.insert("a", start);
    buffer.insert("b", after(start, 1_000));

    buffer.undo(after(start, 2_000));

    assert_eq!(buffer.text(), "a");
}

#[test]
fn never_joins_typing_with_what_is_typed_elsewhere_or_erased() {
    let start = Instant::now();
    let mut typed_then_moved = buffer("");
    typed_then_moved.insert("x", start);
    typed_then_moved.go(Motion::RowStart, false, start);
    typed_then_moved.insert("y", after(start, 100));
    let mut typed_then_erased = buffer("");
    typed_then_erased.insert("x", start);
    typed_then_erased.erase(Motion::Left, after(start, 100));

    typed_then_moved.undo(after(start, 200));
    typed_then_erased.undo(after(start, 200));

    assert_eq!(typed_then_moved.text(), "x");
    assert_eq!(typed_then_erased.text(), "x");
}

#[test]
fn replaces_a_selection_even_in_a_full_text() {
    let start = Instant::now();
    let mut buffer = buffer(&"a".repeat(1_000_000));
    buffer.place(0, 0, false);
    buffer.place(0, 3, true);

    assert!(buffer.insert("xyz", start));

    assert!(buffer.text().starts_with("xyza"));
    assert_eq!(buffer.text().len(), 1_000_000);
}

#[test]
fn undoes_a_run_of_erasures_at_once_and_puts_the_caret_back() {
    let start = Instant::now();
    let mut buffer = buffer("abcdef");
    buffer.go(Motion::RowEnd, false, start);
    buffer.erase(Motion::Left, start);
    buffer.erase(Motion::Left, after(start, 100));
    buffer.go(Motion::RowStart, false, start);
    buffer.erase(Motion::Right, after(start, 2_000));
    buffer.erase(Motion::Right, after(start, 2_100));

    assert_eq!(buffer.text(), "cd");
    buffer.undo(after(start, 3_000));
    assert_eq!(buffer.text(), "abcd");
    assert_eq!(buffer.caret_cell(), (0, 0));
    buffer.undo(after(start, 3_000));
    assert_eq!(buffer.text(), "abcdef");
    assert_eq!(buffer.caret_cell(), (0, 6));
}

#[test]
fn cuts_the_selection_alone() {
    let start = Instant::now();
    let mut buffer = buffer("keep cut");

    assert_eq!(buffer.cut(start), None);
    buffer.place(0, 5, false);
    buffer.go(Motion::RowEnd, true, start);
    assert_eq!(buffer.cut(start), Some("cut".to_owned()));
    assert_eq!(buffer.text(), "keep ");
    buffer.undo(start);
    assert_eq!(buffer.text(), "keep cut");
}

#[test]
fn replaces_everything_in_one_step_keeping_the_caret_where_it_can() {
    let start = Instant::now();
    let mut buffer = buffer("é1234");
    buffer.go(Motion::RowEnd, false, start);

    assert!(!buffer.replace_all("é1234", start));
    assert!(buffer.replace_all("éa", start));
    assert_eq!(buffer.caret_cell(), (0, 2));
    assert!(buffer.replace_all("é", start));
    buffer.go(Motion::RowStart, false, start);
    buffer.go(Motion::RowEnd, false, start);
    assert_eq!(buffer.caret_cell(), (0, 1));
    buffer.undo(start);
    assert_eq!(buffer.text(), "éa");
    buffer.undo(start);
    assert_eq!(buffer.text(), "é1234");
}

#[test]
fn keeps_five_hundred_steps_at_most() {
    let start = Instant::now();
    let mut buffer = buffer("");
    for step in 0..501_u64 {
        buffer.insert("\n", after(start, step));
    }

    let mut undone = 0;
    while buffer.undo(start) {
        undone += 1;
    }

    assert_eq!(undone, 500);
    assert_eq!(buffer.text(), "\n");
}

#[test]
fn keeps_the_caret_on_its_text_when_the_width_changes() {
    let start = Instant::now();
    let mut buffer = buffer("abcdef");
    buffer.go(Motion::RowEnd, false, start);

    buffer.rewrap_to(TextWidth::new(4));

    assert_eq!(buffer.rows().len(), 2);
    assert_eq!(buffer.caret_cell(), (1, 2));
    assert!(buffer.heat(0..6, start).iter().all(|(_, heat)| *heat == 0));
}

#[test]
fn glows_where_it_was_just_typed() {
    let start = Instant::now();
    let mut buffer = buffer("ab");
    buffer.go(Motion::RowEnd, false, start);

    buffer.insert("c", start);

    assert_eq!(buffer.heat(0..3, start), [(0..2, 0), (2..3, u8::MAX)]);
}

#[test]
fn types_into_a_file_longer_than_a_prompt_may_be() {
    let start = Instant::now();
    let mut buffer = buffer(&"a".repeat(1_040_000));

    assert!(buffer.insert("z", start));

    assert_eq!(buffer.text().len(), 1_040_001);
}

#[test]
fn takes_as_many_characters_as_the_largest_file_it_opens() {
    assert_eq!(
        u64::try_from(LONGEST_EDITED).unwrap(),
        dot_api::LARGEST_EDITED
    );
}
