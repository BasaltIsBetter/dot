use std::time::Instant;

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::Caret;
use crate::editor::Touched;
use crate::editor::tests::{after, editor};
use crate::editor::view::tests::{shown, wide};
use dot_transcript::{rows, to_f32};

#[test]
fn a_click_on_the_first_column_or_the_top_row_places_the_caret() {
    let start = Instant::now();
    let mut editor = editor(start);
    shown(&mut editor, after(start, 1_000), Caret::Dark);
    let frame = editor.frame(&wide());

    let at_the_left = editor.press(
        [frame.pane, frame.code_top + frame.row * 1.5],
        (false, 1, start),
        &wide(),
        &IdTranslator,
    );
    let left_cell = editor.buffer.caret_cell();
    let at_the_top = editor.press(
        [frame.code + 1.0, frame.code_top],
        (false, 1, start),
        &wide(),
        &IdTranslator,
    );

    assert_eq!(at_the_left, Pressed::Placed);
    assert_eq!(left_cell, (1, 0));
    assert_eq!(at_the_top, Pressed::Placed);
    assert_eq!(editor.buffer.caret_cell(), (0, 0));
}

#[test]
fn places_the_caret_where_the_code_is_clicked() {
    let start = Instant::now();
    let mut editor = editor(start);
    let layout = wide();
    editor.scene(&layout, start, (None, Caret::Dark), &IdTranslator);

    let placed = editor.press(
        [440.0 + 12.48 * 0.6, 120.0],
        (false, 1, start),
        &layout,
        &IdTranslator,
    );
    let extended = editor.press([440.0, 85.0], (true, 1, start), &layout, &IdTranslator);

    assert_eq!(placed, Pressed::Placed);
    assert_eq!(extended, Pressed::Placed);
    assert_eq!(editor.buffer.selected(), Some("a\nB"));
    assert_eq!(
        editor.press([440.0, 400.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Nothing
    );
    assert_eq!(
        editor.press([440.0, 60.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Nothing
    );
    assert_eq!(
        editor.press([300.0, 120.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Nothing
    );
    assert_eq!(
        editor.press([440.0, 1_760.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Nothing
    );
}

#[test]
fn walks_through_the_versions_from_the_arrows_of_the_bar() {
    let start = Instant::now();
    let mut editor = editor(start);
    let layout = wide();
    let back = 358.0 + 10.56 * 204.5;
    let forward = 358.0 + 10.56 * 207.5;

    assert_eq!(
        editor.press([forward, 40.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Nothing
    );
    assert_eq!(
        editor.press([back, 40.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Changed
    );
    assert_eq!(editor.text(), "a\nb\nc");
    assert_eq!(
        editor.press([forward, 40.0], (false, 1, start), &layout, &IdTranslator),
        Pressed::Changed
    );
    assert_eq!(
        editor.press(
            [358.0 + 10.56 * 220.0, 40.0],
            (false, 1, start),
            &layout,
            &IdTranslator
        ),
        Pressed::Close
    );
    let new = 100.0 + 35.2 * 3.5;
    assert_eq!(
        editor.press([100.0, new], (false, 1, start), &layout, &IdTranslator),
        Pressed::Open("src/new.rs".to_owned())
    );
}

fn opened(text: &str, now: Instant) -> Editor {
    let mut editor = Editor::new(
        "src/a.rs".to_owned(),
        text.to_owned(),
        &Touched::default(),
        Vec::new(),
        now,
    );
    shown(&mut editor, now, Caret::Dark);
    editor
}

fn cell(editor: &Editor, row: usize, column: usize) -> [f32; 2] {
    let frame = editor.frame(&wide());
    [
        frame.x_of(column) + 1.0,
        frame.code_top + frame.row * (to_f32(rows(row)) + 0.5),
    ]
}

fn click(editor: &mut Editor, at: [f32; 2], clicks: u8, now: Instant) -> Pressed {
    editor.press(at, (false, clicks, now), &wide(), &IdTranslator)
}

fn long_file() -> String {
    (1..=100)
        .map(|number| format!("line {number}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_double_click_takes_the_word_and_a_triple_click_the_line() {
    let start = Instant::now();
    let mut editor = opened("let total = 4;\nnext()", start);
    let on_total = cell(&editor, 0, 6);

    assert_eq!(click(&mut editor, on_total, 2, start), Pressed::Placed);
    let word = editor.buffer.selected().map(str::to_owned);
    click(&mut editor, on_total, 3, start);

    assert_eq!(word.as_deref(), Some("total"));
    assert_eq!(editor.buffer.selected(), Some("let total = 4;\n"));
}

#[test]
fn dragging_from_a_click_selects_up_to_the_pointer() {
    let start = Instant::now();
    let mut editor = opened("let total = 4;\nnext()", start);
    let at = cell(&editor, 0, 4);
    click(&mut editor, at, 1, start);

    editor.drag(cell(&editor, 1, 2), &wide());
    let down = editor.buffer.selected().map(str::to_owned);
    editor.drag(cell(&editor, 0, 0), &wide());

    assert_eq!(down.as_deref(), Some("total = 4;\nne"));
    assert_eq!(editor.buffer.selected(), Some("let "));
}

#[test]
fn a_drag_past_the_code_stops_at_the_first_or_last_row_in_sight() {
    let start = Instant::now();
    let mut editor = opened(&long_file(), start);
    let frame = editor.frame(&wide());
    let at = cell(&editor, 2, 0);
    click(&mut editor, at, 1, start);

    editor.drag([frame.code, frame.bottom + 400.0], &wide());
    let below = editor.buffer.caret_cell();
    editor.drag([frame.code, 0.0], &wide());

    assert_eq!(below, (frame.visible() - 1, 0));
    assert_eq!(editor.buffer.caret_cell(), (0, 0));
    assert_eq!(editor.buffer.selected(), Some("line 1\nline 2\n"));
}

#[test]
fn follows_a_drag_held_at_an_edge_a_third_of_a_row_per_frame() {
    let start = Instant::now();
    let mut editor = opened(&long_file(), start);
    let frame = editor.frame(&wide());
    let bottom = [frame.code, frame.bottom - 1.0];
    let middle = [frame.code, frame.code_top + frame.row * 10.0];
    let at = cell(&editor, 2, 0);
    click(&mut editor, at, 1, start);

    let held = [
        editor.follow_edges(bottom, &wide()),
        editor.follow_edges(bottom, &wide()),
    ];
    let top_after_two = editor.top;
    editor.follow_edges(bottom, &wide());
    let top_after_three = editor.top;
    let caret_after_three = editor.buffer.caret_cell().0;
    let in_the_middle = editor.follow_edges(middle, &wide());
    editor.follow_edges([frame.code, frame.code_top + 1.0], &wide());
    editor.follow_edges([frame.code, frame.code_top + 1.0], &wide());
    let up_again = editor.top;
    editor.follow_edges([frame.code, frame.code_top + 1.0], &wide());

    assert_eq!(held, [true, true]);
    assert_eq!(top_after_two, 0);
    assert_eq!(top_after_three, 1);
    assert_eq!(caret_after_three, frame.visible());
    assert!(!in_the_middle);
    assert_eq!(up_again, 1);
    assert_eq!(editor.top, 0);
}

#[test]
fn neither_drags_nor_follows_an_edge_once_let_go_or_after_a_click_beside_the_code() {
    let start = Instant::now();
    let mut editor = opened(&long_file(), start);
    let frame = editor.frame(&wide());
    let bottom = [frame.code, frame.bottom - 1.0];
    let at = cell(&editor, 2, 0);
    click(&mut editor, at, 1, start);

    editor.release();
    editor.drag(cell(&editor, 5, 3), &wide());
    let let_go = (
        editor.follow_edges(bottom, &wide()),
        editor.buffer.selection(),
    );
    let at = cell(&editor, 2, 0);
    click(&mut editor, at, 1, start);
    click(
        &mut editor,
        [frame.pane - 10.0, frame.code_top + 2.0],
        1,
        start,
    );
    editor.drag(cell(&editor, 5, 3), &wide());

    assert_eq!(let_go, (false, None));
    assert!(!editor.follow_edges(bottom, &wide()));
    assert_eq!(editor.buffer.selection(), None);
    assert_eq!(editor.top, 0);
}
