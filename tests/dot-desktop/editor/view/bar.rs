use std::time::Instant;

use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::editor::tests::editor;
use crate::editor::{Gesture, Touched};
use dot_api::WorkspaceError;

pub(crate) fn wide() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
}

pub(crate) fn bar_of(editor: &Editor) -> (Bar, Frame) {
    let frame = Frame::of(&wide(), editor.buffer.text().split('\n').count());
    (Bar::of(&frame, &wide(), &IdTranslator), frame)
}

pub(crate) fn texts(pane: &Pane) -> Vec<(u16, String)> {
    pane.lines
        .iter()
        .map(|placed| {
            let text = placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            (placed.line.indent, text)
        })
        .collect()
}

pub(crate) fn inks(pane: &Pane, line: usize) -> Vec<Ink> {
    pane.lines[line]
        .line
        .spans
        .iter()
        .map(|span| span.ink)
        .collect()
}

const fn column(at: f32) -> f32 {
    358.0 + 10.56 * at
}

#[test]
fn sets_the_arrows_and_the_way_back_at_the_right_of_the_bar() {
    let (bar, _) = bar_of(&editor(Instant::now()));

    assert_eq!([bar.left, bar.advance], [358.0, 10.56]);
    assert_eq!(
        [bar.columns, bar.back, bar.forward, bar.close],
        [235, 204, 207, 210]
    );
    assert_eq!(bar.tree, Some([38.0, 90.0, 294.0]));
}

#[test]
fn shows_the_path_what_claude_changed_and_where_each_arrow_leads() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);

    let pane = bar.pane(&editor, &frame, None, &IdTranslator);

    assert_eq!(
        texts(&pane),
        [
            (0, "src/a.rs  editor-by-claude 1 tool-lines".to_owned()),
            (204, "↶".to_owned()),
            (207, "↷".to_owned()),
            (210, "help-stop-key editor-back".to_owned()),
        ]
    );
    assert_eq!(
        inks(&pane, 0),
        [Ink::Receding, Ink::Full, Ink::Secondary, Ink::Secondary]
    );
    assert_eq!(inks(&pane, 1), [Ink::Secondary]);
    assert_eq!(inks(&pane, 2), [Ink::Faint]);
    assert_eq!(inks(&pane, 3), [Ink::Secondary, Ink::Secondary]);
    assert_eq!(pane.left, 358.0);
    assert!(
        pane.lines
            .iter()
            .all(|placed| (placed.top - 29.0).abs() < f32::EPSILON)
    );
}

#[test]
fn lights_the_way_back_under_the_pointer_like_the_prototype() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);
    let close = column(f32::from(u16::try_from(bar.close).unwrap()) + 0.5);

    let pane = bar.pane(&editor, &frame, Some([close, 40.0]), &IdTranslator);

    assert_eq!(bar.hit(&editor, [close, 40.0], &frame), Some(Hit::Close));
    assert_eq!(inks(&pane, 3), [Ink::Full, Ink::Full]);
}

#[test]
fn lights_the_arrow_under_the_pointer_and_says_where_it_leads() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);

    let pane = bar.pane(&editor, &frame, Some([column(204.5), 40.0]), &IdTranslator);

    assert_eq!(
        texts(&pane)[0].1,
        "src/a.rs  editor-by-claude 1 tool-lines  editor-to-before"
    );
    assert_eq!(inks(&pane, 1), [Ink::Full]);
    let unlit = bar.pane(&editor, &frame, Some([column(207.5), 40.0]), &IdTranslator);
    assert_eq!(
        texts(&unlit)[0].1,
        "src/a.rs  editor-by-claude 1 tool-lines"
    );
}

#[test]
fn counts_your_lines_and_those_of_a_file_claude_created() {
    let start = Instant::now();
    let mut yours = editor(start);
    yours.act(Gesture::Insert("x".to_owned()), start, || None);
    let mut touched = Touched::default();
    touched.record(dot_api::FileChange::between("new.rs", "", "a\nb\n", true));
    let created = Editor::new(
        "new.rs".to_owned(),
        "a\nb\n".to_owned(),
        &touched,
        Vec::new(),
        start,
    );

    assert_eq!(
        Bar::counts(&yours, &IdTranslator),
        "editor-by-claude 1 tool-lines · editor-by-you 1 tool-lines"
    );
    assert_eq!(
        Bar::counts(&created, &IdTranslator),
        "editor-created-by-claude 3 tool-lines"
    );
}

#[test]
fn finds_the_arrows_the_way_back_and_the_files_claude_touched() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);
    let tree_row = |row: f32| 90.0 + 35.2 * (row + 0.5);

    assert_eq!(
        bar.hit(&editor, [column(204.5), 40.0], &frame),
        Some(Hit::Back)
    );
    assert_eq!(
        bar.hit(&editor, [column(207.5), 40.0], &frame),
        Some(Hit::Forward)
    );
    assert_eq!(
        bar.hit(&editor, [column(210.5), 40.0], &frame),
        Some(Hit::Close)
    );
    assert_eq!(
        bar.hit(&editor, [column(234.5), 40.0], &frame),
        Some(Hit::Close)
    );
    assert_eq!(bar.hit(&editor, [column(235.5), 40.0], &frame), None);
    assert_eq!(bar.hit(&editor, [column(205.5), 40.0], &frame), None);
    assert_eq!(bar.hit(&editor, [300.0, 40.0], &frame), None);
    assert_eq!(bar.hit(&editor, [column(204.5), 80.0], &frame), None);
    assert_eq!(
        bar.hit(&editor, [100.0, tree_row(3.0)], &frame),
        Some(Hit::Open("src/new.rs".to_owned()))
    );
    assert_eq!(bar.hit(&editor, [100.0, tree_row(1.0)], &frame), None);
    assert_eq!(bar.hit(&editor, [100.0, tree_row(2.0)], &frame), None);
    assert_eq!(bar.hit(&editor, [100.0, tree_row(0.0)], &frame), None);
    assert_eq!(bar.hit(&editor, [100.0, 85.0], &frame), None);
    assert_eq!(bar.hit(&editor, [300.0, tree_row(3.0)], &frame), None);
    assert_eq!(bar.hit(&editor, [100.0, tree_row(9.0)], &frame), None);
}

#[test]
fn cuts_what_does_not_fit_before_the_arrows() {
    let spans = vec![
        Span::new("abcdef", Ink::Full),
        Span::new("ghi", Ink::Secondary),
    ];

    assert_eq!(
        fit(spans.clone(), 7)
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>(),
        ["abcdef", "…"]
    );
    assert_eq!(
        fit(spans, 3)
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>(),
        ["ab…"]
    );
}

#[test]
fn tells_the_status_bar_whether_the_file_is_saved() {
    let mut editor = editor(Instant::now());

    assert_eq!(activity(&editor, &IdTranslator), "a.rs · editor-saved");
    editor.saved(Err(WorkspaceError::TooLarge));
    assert_eq!(activity(&editor, &IdTranslator), "a.rs · editor-not-saved");
}

#[test]
fn names_where_each_arrow_leads() {
    assert_eq!(text_of(Toward::Claude), Text::EditorToClaude);
    assert_eq!(text_of(Toward::Before), Text::EditorToBefore);
    assert_eq!(text_of(Toward::Yours), Text::EditorToYours);
}

#[test]
fn names_a_file_at_the_root_without_a_folder() {
    let editor = Editor::new(
        "top.rs".to_owned(),
        String::new(),
        &Touched::default(),
        Vec::new(),
        Instant::now(),
    );

    assert_eq!(Bar::path(&editor).len(), 1);
}

#[test]
fn shows_invisible_characters_of_the_path_instead_of_hiding_them() {
    let editor = Editor::new(
        "s\u{202e}rc/a\u{200b}.rs".to_owned(),
        "a".to_owned(),
        &Touched::default(),
        Vec::new(),
        Instant::now(),
    );

    let path: String = Bar::path(&editor)
        .iter()
        .map(|span| span.text.as_str())
        .collect();

    assert_eq!(path, "s\u{fffd}rc/a\u{fffd}.rs");
}
