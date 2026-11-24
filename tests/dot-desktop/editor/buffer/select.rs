use dot_transcript::TextWidth;

use super::*;

fn buffer(text: &str) -> Buffer {
    Buffer::new(text.to_owned(), TextWidth::new(40))
}

#[test]
fn selects_the_word_or_the_line_under_a_cell() {
    let mut buffer = buffer("let total = 4;\nnext()");

    let word = buffer.select_word(0, 6);
    let selected_word = buffer.selected().map(str::to_owned);
    let line = buffer.select_line(0, 6);

    assert_eq!(word, 4..9);
    assert_eq!(selected_word.as_deref(), Some("total"));
    assert_eq!(line, 0..15);
    assert_eq!(buffer.selected(), Some("let total = 4;\n"));
    assert_eq!(buffer.caret_cell(), (1, 0));
}

#[test]
fn drags_from_where_a_click_held_in_both_directions() {
    let mut buffer = buffer("alpha beta gamma");
    buffer.place(0, 6, false);
    let held = buffer.fixed_end()..buffer.fixed_end();

    buffer.drag_to(0, 16, &held);
    let forward = buffer.selected().map(str::to_owned);
    buffer.drag_to(0, 2, &held);
    let backward = buffer.selected().map(str::to_owned);
    buffer.drag_to(0, 6, &held);

    assert_eq!(forward.as_deref(), Some("beta gamma"));
    assert_eq!(backward.as_deref(), Some("pha "));
    assert_eq!(buffer.selection(), None);
}

#[test]
fn keeps_a_held_word_whole_whichever_way_the_drag_goes() {
    let mut buffer = buffer("alpha beta gamma");
    let held = buffer.select_word(0, 7);

    buffer.drag_to(0, 8, &held);
    let inside = buffer.selected().map(str::to_owned);
    buffer.drag_to(0, 14, &held);
    let after = buffer.selected().map(str::to_owned);
    let after_caret = buffer.caret_cell();
    buffer.drag_to(0, 1, &held);

    assert_eq!(inside.as_deref(), Some("beta"));
    assert_eq!(after.as_deref(), Some("beta gam"));
    assert_eq!(after_caret, (0, 14));
    assert_eq!(buffer.selected(), Some("lpha beta"));
    assert_eq!(buffer.caret_cell(), (0, 1));
}

#[test]
fn holds_the_anchor_of_a_shift_click_or_else_the_caret() {
    let mut buffer = buffer("alpha beta");
    buffer.place(0, 2, false);
    let caret = buffer.fixed_end();
    buffer.place(0, 8, true);

    assert_eq!(caret, 2);
    assert_eq!(buffer.fixed_end(), 2);
}
