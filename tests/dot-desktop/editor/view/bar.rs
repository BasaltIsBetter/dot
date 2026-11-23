use std::time::Instant;

use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::editor::tests::editor;
use crate::editor::{Gesture, Touched};
use dot_api::WorkspaceError;

fn wide() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
}

fn bar_of(editor: &Editor) -> (Bar, Frame) {
    let frame = Frame::of(&wide(), editor.buffer.text().split('\n').count());
    (Bar::of(&frame, &wide(), &IdTranslator), frame)
}

fn texts(pane: &Pane) -> Vec<(u16, String)> {
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

fn inks(pane: &Pane, line: usize) -> Vec<Ink> {
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
    assert_eq!(inks(&pane, 3), [Ink::Full, Ink::Secondary]);
    assert_eq!(pane.left, 358.0);
    assert!(
        pane.lines
            .iter()
            .all(|placed| (placed.top - 29.0).abs() < f32::EPSILON)
    );
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
    touched.record(&dot_api::FileChange::between("new.rs", "", "a\nb\n", true));
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
fn names_a_folder_outside_the_session_and_opens_what_claude_touched_there() {
    let folder = "/Users/someone/Desktop/tool";
    let mut touched = Touched::default();
    for name in ["lib.py", "run.py"] {
        touched.record(&dot_api::FileChange::between(
            &format!("{folder}/{name}"),
            "a\n",
            "b\n",
            false,
        ));
    }
    let editor = Editor::new(
        format!("{folder}/run.py"),
        "b".to_owned(),
        &touched,
        vec![
            "lib.py".to_owned(),
            "old.py".to_owned(),
            "run.py".to_owned(),
        ],
        Instant::now(),
    );
    let (bar, frame) = bar_of(&editor);
    let tree_row = |row: f32| 90.0 + 35.2 * (row + 0.5);

    let tree = bar.tree(&editor, &wide(), &frame).unwrap();

    assert_eq!(texts(&tree)[0], (0, "tool/".to_owned()));
    assert_eq!(
        bar.hit(&editor, [100.0, tree_row(1.0)], &frame),
        Some(Hit::Open(format!("{folder}/lib.py")))
    );
    assert_eq!(bar.hit(&editor, [100.0, tree_row(2.0)], &frame), None);
    assert_eq!(bar.hit(&editor, [100.0, tree_row(3.0)], &frame), None);
}

#[test]
fn lists_the_folder_with_what_claude_touched_marked() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);

    let tree = bar.tree(&editor, &wide(), &frame).unwrap();

    assert_eq!(
        texts(&tree),
        [
            (0, "src/".to_owned()),
            (2, "✎ a.rs".to_owned()),
            (2, "  mod.rs".to_owned()),
            (2, "✚ new.rs".to_owned()),
            (2, "  tests/".to_owned()),
        ]
    );
    assert_eq!(inks(&tree, 0), [Ink::Receding]);
    assert_eq!(inks(&tree, 1), [Ink::Full, Ink::Full]);
    assert_eq!(inks(&tree, 2), [Ink::Full, Ink::Receding]);
    assert_eq!(inks(&tree, 3), [Ink::Full, Ink::Secondary]);
    let tops: Vec<f32> = tree.lines.iter().map(|placed| placed.top).collect();
    assert_eq!(tops, [93.0, 128.0, 163.0, 199.0, 234.0]);
    assert_eq!(tree.left, 38.0);
}

#[test]
fn names_the_folder_itself_and_has_no_tree_in_a_narrow_window() {
    let start = Instant::now();
    let editor = Editor::new(
        "top.rs".to_owned(),
        String::new(),
        &Touched::default(),
        Vec::new(),
        start,
    );
    let (bar, frame) = bar_of(&editor);
    let narrow = Layout::reflowed(PhysicalSize::new(1000, 700), 2.0);
    let narrow_frame = Frame::of(&narrow, 1);

    assert_eq!(
        texts(&bar.tree(&editor, &wide(), &frame).unwrap())[0].1,
        "./"
    );
    assert_eq!(
        Bar::of(&narrow_frame, &narrow, &IdTranslator).tree(&editor, &narrow, &narrow_frame),
        None
    );
    assert_eq!(Bar::path(&editor).len(), 1);
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
fn names_where_each_arrow_leads_and_marks_each_kind_of_file() {
    assert_eq!(text_of(Toward::Claude), Text::EditorToClaude);
    assert_eq!(text_of(Toward::Before), Text::EditorToBefore);
    assert_eq!(text_of(Toward::Yours), Text::EditorToYours);
    assert_eq!(mark_of(true), "✚ ");
    assert_eq!(mark_of(false), "✎ ");
}

#[test]
fn titles_the_tree_with_the_folder_s_own_name() {
    let start = Instant::now();
    let deep = Editor::new(
        "crates/dot/src/a.rs".to_owned(),
        String::new(),
        &Touched::default(),
        Vec::new(),
        start,
    );
    let (bar, frame) = bar_of(&deep);

    assert_eq!(
        texts(&bar.tree(&deep, &wide(), &frame).unwrap())[0].1,
        "src/"
    );
}
