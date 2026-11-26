use std::time::Instant;

use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::Motion;
use crate::editor::tests::{after, editor};
use crate::editor::{Gesture, Touched};

pub(super) fn wide() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
}

fn narrow() -> Layout {
    Layout::reflowed(PhysicalSize::new(1000, 700), 2.0)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

fn texts(pane: &Pane) -> Vec<String> {
    pane.lines
        .iter()
        .map(|placed| {
            let spans: String = placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            format!("{}{spans}", " ".repeat(usize::from(placed.line.indent)))
        })
        .collect()
}

pub(super) fn shown(editor: &mut Editor, now: Instant, caret: Caret) -> Scene {
    editor.scene(&wide(), now, (None, caret), &IdTranslator)
}

#[test]
fn places_the_tree_the_numbers_and_the_code_side_by_side() {
    let frame = Frame::of(&wide(), 120);

    assert_eq!(frame.tree, Some(320.0));
    assert_eq!(
        [frame.pane, frame.bar_top, frame.numbers],
        [320.0, 29.0, 346.0]
    );
    assert_eq!(
        [frame.rule, frame.code, frame.right, frame.bottom],
        [443.0, 465.0, 2842.0, 1755.0]
    );
    assert_eq!(frame.digits, 3);
    assert!(close(frame.code_top, 79.8) && close(frame.row, 35.2) && close(frame.advance, 12.48));
    assert_eq!(frame.width(), TextWidth::new(190));
    assert_eq!(frame.visible(), 47);
    assert_eq!([frame.row_top(0), frame.row_top(1)], [80.0, 115.0]);
    assert!(close(frame.x_of(2), 489.96));
}

#[test]
fn leaves_the_tree_out_of_a_narrow_window() {
    let frame = Frame::of(&narrow(), 9);

    assert_eq!(frame.tree, None);
    assert_eq!(
        [frame.pane, frame.numbers, frame.rule, frame.code],
        [0.0, 26.0, 98.0, 120.0]
    );
    assert_eq!(frame.digits, 1);
    assert_eq!(frame.width(), TextWidth::new(67));
    assert_eq!(frame.visible(), 16);
    assert_eq!(Frame::of(&narrow(), 10).digits, 2);
    assert_eq!(Frame::of(&narrow(), 0).digits, 1);
}

#[test]
fn keeps_a_few_columns_and_one_row_in_the_smallest_room() {
    let frame = Frame::of(&wide(), 1);
    let cramped = Frame {
        right: frame.code + 10.0,
        bottom: frame.code_top - 5.0,
        ..frame
    };

    assert_eq!(cramped.width(), TextWidth::new(8));
    assert_eq!(cramped.visible(), 1);
}

#[test]
fn numbers_each_line_and_marks_the_ones_claude_changed() {
    let start = Instant::now();
    let mut editor = editor(start);

    let scene = shown(&mut editor, after(start, 1_000), Caret::Lit);

    assert_eq!(texts(&scene.panes[0]), ["   1", "── 2", "   3"]);
    let inks: Vec<Ink> = scene.panes[0]
        .lines
        .iter()
        .map(|placed| placed.line.spans[0].ink)
        .collect();
    assert_eq!(inks, [Ink::Faint, Ink::Full, Ink::Faint]);
    assert_eq!(texts(&scene.panes[1]), ["a", "B", "c", "_"]);
    assert_eq!(scene.panes[1].lines[3].line.spans[0].ink, Ink::Live);
    assert_eq!(scene.panes[0].lines[1].top, 115.0);
    assert_eq!(scene.panes[1].left, 440.0);
    assert!(!scene.animating);
}

#[test]
fn marks_in_orange_the_lines_you_change() {
    let start = Instant::now();
    let mut editor = editor(start);
    editor.act(Gesture::Insert("x".to_owned()), start, || None);

    let scene = shown(&mut editor, start, Caret::Dark);

    assert_eq!(texts(&scene.panes[0])[0], "── 1");
    assert_eq!(scene.panes[0].lines[0].line.spans[0].heat, u8::MAX);
    assert_eq!(texts(&scene.panes[1]), ["xa", "B", "c"]);
    assert!(
        scene.panes[1].lines[0]
            .line
            .spans
            .iter()
            .any(|span| span.heat > 0)
    );
    assert!(scene.animating);
}

#[test]
fn draws_the_caret_dimmed_in_the_background_and_not_at_all_when_dark() {
    let start = Instant::now();
    let mut editor = editor(start);

    let background = shown(&mut editor, after(start, 1_000), Caret::Receded);
    let dark = shown(&mut editor, after(start, 1_000), Caret::Dark);

    assert_eq!(
        background.panes[1].lines[3].line.spans[0].ink,
        Ink::Receding
    );
    assert_eq!(texts(&dark.panes[1]), ["a", "B", "c"]);
}

#[test]
fn inks_keywords_and_turns_tabs_into_four_spaces() {
    let start = Instant::now();
    let mut editor = Editor::new(
        "a.rs".to_owned(),
        "\tfn x".to_owned(),
        &Touched::default(),
        Vec::new(),
        start,
    );

    let scene = shown(&mut editor, after(start, 1_000), Caret::Dark);

    let spans = &scene.panes[1].lines[0].line.spans;
    assert_eq!(spans[0].text, "    ");
    assert_eq!(spans[1].text, "fn ");
    assert_eq!(spans[1].ink, Ink::Secondary);
    assert_eq!(spans[2].ink, Ink::Full);
}

fn edited(text: &str, now: Instant) -> Editor {
    Editor::new(
        "a.rs".to_owned(),
        text.to_owned(),
        &Touched::default(),
        Vec::new(),
        now,
    )
}

#[test]
fn inks_the_words_of_a_line_where_they_are_in_the_file() {
    let start = Instant::now();
    let mut editor = edited("ab\nx fn", start);

    let scene = shown(&mut editor, after(start, 1_000), Caret::Dark);

    let spans = &scene.panes[1].lines[1].line.spans;
    let keyword = spans.iter().find(|span| span.text.contains("fn")).unwrap();
    assert_eq!(keyword.ink, Ink::Secondary);
    assert_eq!(spans[0].ink, Ink::Full);
}

#[test]
fn draws_the_caret_only_on_a_row_in_sight() {
    let start = Instant::now();
    let text: String = (1..=100).fold(String::new(), |mut text, line| {
        text.push_str(&line.to_string());
        text.push('\n');
        text
    });
    let mut editor = edited(&text, start);
    shown(&mut editor, after(start, 1_000), Caret::Lit);
    editor.buffer.place(90, 0, false);

    editor.top = 90 - editor.visible;
    let just_below = shown(&mut editor, after(start, 1_000), Caret::Lit);
    editor.top += 1;
    let on_the_last_row = shown(&mut editor, after(start, 1_000), Caret::Lit);

    assert!(!texts(&just_below.panes[1]).contains(&"_".to_owned()));
    assert!(texts(&on_the_last_row.panes[1]).contains(&"_".to_owned()));
}

#[test]
fn washes_no_row_after_a_selection_that_ends_with_a_line_break() {
    let start = Instant::now();
    let mut editor = edited("ab\ncd", start);
    editor.buffer.place(0, 0, false);
    editor.buffer.place(1, 0, true);

    let scene = shown(&mut editor, after(start, 1_000), Caret::Dark);

    assert_eq!(scene.grounds.len(), 2);
}

#[test]
fn lays_a_band_on_the_caret_row_and_the_rules_beside_the_code() {
    let start = Instant::now();
    let mut editor = editor(start);
    editor.act(Gesture::Go(Motion::Down, false), start, || None);

    let scene = shown(&mut editor, after(start, 1_000), Caret::Dark);

    let band = scene.grounds[0];
    assert_eq!([band.left, band.top], [418.0, 115.0]);
    assert!(close(band.width, 2842.0 + 12.48 - 418.0) && close(band.height, 35.2));
    assert_eq!(band.color, Paint::ink(Ink::Trace));
    assert!(
        scene.marks[..2]
            .iter()
            .all(|rule| rule.color == Paint::ink(Ink::Trace))
    );
    let rules: Vec<[f32; 2]> = scene
        .marks
        .iter()
        .map(|rule| [rule.left, rule.top])
        .collect();
    assert_eq!(rules[0][0], 418.0);
    assert!(close(rules[0][1], 79.8));
    assert_eq!(rules[1], [320.0, 0.0]);
    assert!(
        scene
            .marks
            .iter()
            .all(|rule| (rule.width - 2.0).abs() < f32::EPSILON)
    );
    assert!(close(scene.marks[0].height, 1755.0 - 79.8));
    assert!(close(scene.marks[1].height, 1755.0));
}

#[test]
fn washes_the_selection_in_the_live_colour_through_the_line_breaks() {
    let start = Instant::now();
    let mut editor = editor(start);
    editor.act(Gesture::SelectAll, start, || None);

    let scene = shown(&mut editor, after(start, 1_000), Caret::Dark);

    let wash: Vec<[f32; 2]> = scene.grounds[1..]
        .iter()
        .map(|rectangle| [rectangle.left, rectangle.top])
        .collect();
    assert_eq!(wash, [[440.0, 80.0], [440.0, 115.0], [440.0, 150.0]]);
    assert!(close(scene.grounds[1].width, 2.0 * 12.48));
    assert!(close(scene.grounds[3].width, 12.48));
    assert_eq!(scene.grounds[1].color.rgb(), [64, 43, 97]);
}

#[test]
fn gives_the_window_of_accents_the_caret_cell() {
    let start = Instant::now();
    let mut editor = editor(start);
    editor.act(Gesture::Go(Motion::Down, false), start, || None);
    editor.act(Gesture::Go(Motion::RowEnd, false), start, || None);

    let area = editor.caret_area(&wide());

    assert!(close(area.left, 440.0 + 12.48));
    assert_eq!(area.top, 115.0);
    assert!(close(area.width, 12.48) && close(area.height, 35.2));
}
