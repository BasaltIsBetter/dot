use std::time::Instant;

use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::editor::Touched;
use crate::editor::tests::editor;
use crate::editor::view::bar::Hit;
use crate::editor::view::bar::tests::{bar_of, inks, texts, wide};

#[test]
fn names_a_folder_outside_the_session_and_opens_what_claude_touched_there() {
    let folder = "/Users/someone/Desktop/tool";
    let mut touched = Touched::default();
    for name in ["lib.py", "run.py"] {
        touched.record(dot_api::FileChange::between(
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

    let tree = bar.tree(&editor, &wide(), &frame, None).unwrap();

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

    let tree = bar.tree(&editor, &wide(), &frame, None).unwrap();

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
        texts(&bar.tree(&editor, &wide(), &frame, None).unwrap())[0].1,
        "./"
    );
    assert_eq!(
        Bar::of(&narrow_frame, &narrow, &IdTranslator).tree(&editor, &narrow, &narrow_frame, None),
        None
    );
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
        texts(&bar.tree(&deep, &wide(), &frame, None).unwrap())[0].1,
        "src/"
    );
}

#[test]
fn shows_invisible_characters_of_a_name_instead_of_hiding_them() {
    let editor = Editor::new(
        "s\u{202e}rc/a\u{200b}.rs".to_owned(),
        "a".to_owned(),
        &Touched::default(),
        vec!["a\u{200b}.rs".to_owned(), "b\u{2066}.rs".to_owned()],
        Instant::now(),
    );
    let (bar, frame) = bar_of(&editor);

    let tree = bar.tree(&editor, &wide(), &frame, None).unwrap();

    assert_eq!(texts(&tree)[0].1, "s\u{fffd}rc/");
    assert!(
        texts(&tree)
            .iter()
            .any(|(_, name)| name.ends_with("a\u{fffd}.rs"))
    );
    assert!(
        texts(&tree)
            .iter()
            .any(|(_, name)| name.ends_with("b\u{fffd}.rs"))
    );
}

#[test]
fn marks_each_kind_of_file() {
    assert_eq!(mark_of(true), "✚ ");
    assert_eq!(mark_of(false), "✎ ");
}

#[test]
fn draws_a_huge_folder_as_its_first_entries_alone() {
    let start = Instant::now();
    let listed = |count: usize| (0..count).map(|entry| format!("f{entry}.rs")).collect();
    let open = |count: usize| {
        Editor::new(
            "src/a.rs".to_owned(),
            String::new(),
            &Touched::default(),
            listed(count),
            start,
        )
    };
    let (huge, few) = (open(100_000), open(60));
    let (bar, frame) = bar_of(&huge);

    let tree = bar.tree(&huge, &wide(), &frame, None).unwrap();

    assert!(tree.lines.len() < 60);
    assert_eq!(Some(tree), bar.tree(&few, &wide(), &frame, None));
}

#[test]
fn lights_a_file_claude_touched_under_the_pointer_like_the_prototype() {
    let editor = editor(Instant::now());
    let (bar, frame) = bar_of(&editor);
    let hovered = bar.hit(&editor, [100.0, 90.0 + 35.2 * 3.5], &frame);
    assert!(matches!(hovered, Some(Hit::Open(_))));

    let lit = bar
        .tree(&editor, &wide(), &frame, hovered.as_ref())
        .unwrap();

    assert_eq!(inks(&lit, 3), [Ink::Full, Ink::Full]);
    assert_eq!(inks(&lit, 2), [Ink::Full, Ink::Receding]);
    let elsewhere = Hit::Open("src/mod.rs".to_owned());
    let unlit = bar
        .tree(&editor, &wide(), &frame, Some(&elsewhere))
        .unwrap();
    assert_eq!(inks(&unlit, 3), [Ink::Full, Ink::Secondary]);
    assert_eq!(inks(&unlit, 2), [Ink::Full, Ink::Receding]);
}
