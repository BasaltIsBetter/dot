use std::sync::Mutex;
use std::time::Duration;

use dot_api::{Hunk, PatchLine, Workspace};
use dot_transcript::Ink;

use crate::canvas::{Rectangle, Rgba};

use super::*;

#[derive(Debug, Default)]
pub(crate) struct MemoryWorkspace {
    files: Mutex<BTreeMap<String, String>>,
    refuses_writes: bool,
}

impl MemoryWorkspace {
    pub(crate) fn with(files: &[(&str, &str)]) -> Self {
        let files = files
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect();
        Self {
            files: Mutex::new(files),
            refuses_writes: false,
        }
    }

    pub(crate) fn refusing(self) -> Self {
        Self {
            refuses_writes: true,
            ..self
        }
    }

    pub(crate) fn text(&self, path: &str) -> Option<String> {
        self.files.lock().unwrap().get(path).cloned()
    }
}

impl Workspace for MemoryWorkspace {
    fn read(&self, path: &str) -> Result<String, WorkspaceError> {
        self.text(path).ok_or(WorkspaceError::NotAFile)
    }

    fn write(&self, path: &str, text: &str) -> Result<(), WorkspaceError> {
        if self.refuses_writes {
            return Err(WorkspaceError::Failed("disk full".to_owned()));
        }
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), text.to_owned());
        Ok(())
    }

    fn list(&self, folder: &str) -> Result<Vec<String>, WorkspaceError> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .keys()
            .filter(|path| folder_of(path) == folder)
            .map(|path| path.rsplit('/').next().unwrap().to_owned())
            .collect())
    }
}

pub(crate) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(crate) fn edited(path: &str, new_start: u64, lines: Vec<PatchLine>) -> FileChange {
    FileChange {
        path: path.to_owned(),
        created: false,
        hunks: vec![Hunk {
            old_start: new_start,
            new_start,
            lines,
        }],
        siblings: None,
    }
}

pub(crate) fn same(text: &str) -> PatchLine {
    PatchLine::Same(text.to_owned())
}

pub(crate) fn removed(text: &str) -> PatchLine {
    PatchLine::Removed(text.to_owned())
}

pub(crate) fn added(text: &str) -> PatchLine {
    PatchLine::Added(text.to_owned())
}

pub(crate) fn touched_by_claude() -> Touched {
    let mut touched = Touched::default();
    touched.record(&edited(
        "src/a.rs",
        1,
        vec![same("a"), removed("b"), added("B"), same("c")],
    ));
    touched.record(&FileChange {
        created: true,
        ..edited("src/new.rs", 1, vec![added("x")])
    });
    touched
}

pub(crate) fn editor(now: Instant) -> Editor {
    let listed = vec![
        "a.rs".to_owned(),
        "mod.rs".to_owned(),
        "new.rs".to_owned(),
        "tests/".to_owned(),
    ];
    Editor::new(
        "src/a.rs".to_owned(),
        "a\nB\nc".to_owned(),
        &touched_by_claude(),
        listed,
        now,
    )
}

#[test]
fn records_the_changes_of_each_file_inside_the_folder_or_out_of_it() {
    let mut touched = touched_by_claude();
    touched.record(&edited("/elsewhere/x.rs", 1, vec![added("x")]));
    touched.record(&edited("src/a.rs", 3, vec![same("c"), added("d")]));

    assert_eq!(touched.created("src/a.rs"), Some(false));
    assert_eq!(touched.created("src/new.rs"), Some(true));
    assert_eq!(touched.created("/elsewhere/x.rs"), Some(false));
    assert_eq!(touched.created("/elsewhere/y.rs"), None);
    assert_eq!(touched.files["src/a.rs"].changes.len(), 2);
    assert_eq!(touched.files["/elsewhere/x.rs"].changes.len(), 1);
    assert_eq!(touched.files["src/new.rs"].changes, Vec::<Vec<Hunk>>::new());
}

#[test]
fn opens_a_file_outside_the_folder_among_its_own_neighbours() {
    let start = Instant::now();
    let mut touched = Touched::default();
    touched.record(&FileChange {
        created: true,
        ..edited("/Users/someone/Desktop/tool/run.py", 1, vec![added("x")])
    });
    touched.record(&edited(
        "/Users/someone/Desktop/tool/lib.py",
        1,
        vec![removed("a"), added("b")],
    ));

    let editor = Editor::new(
        "/Users/someone/Desktop/tool/run.py".to_owned(),
        "x".to_owned(),
        &touched,
        vec![
            "lib.py".to_owned(),
            "run.py".to_owned(),
            "old.py".to_owned(),
        ],
        start,
    );

    assert_eq!(editor.folder, "/Users/someone/Desktop/tool");
    let marks: Vec<(Option<bool>, &str)> = editor
        .entries
        .iter()
        .map(|entry| (entry.touched, entry.path.as_str()))
        .collect();
    assert_eq!(
        marks,
        [
            (Some(false), "/Users/someone/Desktop/tool/lib.py"),
            (Some(true), "/Users/someone/Desktop/tool/run.py"),
            (None, "/Users/someone/Desktop/tool/old.py"),
        ]
    );
    assert!(editor.versions.created());
}

#[test]
fn a_file_created_again_forgets_its_earlier_changes() {
    let mut touched = touched_by_claude();

    touched.record(&FileChange {
        created: true,
        ..edited("src/a.rs", 1, vec![added("x")])
    });

    assert_eq!(touched.created("src/a.rs"), Some(true));
    assert_eq!(touched.files["src/a.rs"].changes, Vec::<Vec<Hunk>>::new());
}

#[test]
fn opens_a_file_with_its_neighbours_marked_by_what_claude_did() {
    let start = Instant::now();
    let editor = editor(start);

    assert_eq!(editor.path(), "src/a.rs");
    assert_eq!(editor.text(), "a\nB\nc");
    assert_eq!(editor.folder, "src");
    let marks: Vec<(&str, Option<bool>, &str)> = editor
        .entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.touched, entry.path.as_str()))
        .collect();
    assert_eq!(
        marks,
        [
            ("a.rs", Some(false), "src/a.rs"),
            ("mod.rs", None, "src/mod.rs"),
            ("new.rs", Some(true), "src/new.rs"),
            ("tests/", None, "src/tests/"),
        ]
    );
    assert_eq!(editor.versions.claude_count(), 1);
}

#[test]
fn names_entries_of_the_folder_itself_without_a_slash_in_front() {
    let start = Instant::now();
    let editor = Editor::new(
        "top.rs".to_owned(),
        String::new(),
        &Touched::default(),
        vec!["top.rs".to_owned()],
        start,
    );

    assert_eq!(editor.folder, "");
    assert_eq!(editor.entries[0].path, "top.rs");
    assert_eq!(editor.entries[0].touched, None);
}

#[test]
fn says_whether_the_last_keystroke_was_saved() {
    let mut editor = editor(Instant::now());

    editor.saved(Err(WorkspaceError::TooLarge));
    assert_eq!(editor.saving(), &Saving::Failed(WorkspaceError::TooLarge));
    editor.saved(Ok(()));
    assert_eq!(editor.saving(), &Saving::Saved);
}

#[test]
fn scrolls_by_whole_rows_within_the_text() {
    let start = Instant::now();
    let mut editor = Editor::new(
        "a.rs".to_owned(),
        "1\n2\n3\n4\n5".to_owned(),
        &Touched::default(),
        Vec::new(),
        start,
    );

    editor.scroll(2.4);
    assert_eq!(editor.top, 2);
    editor.scroll(10.0);
    assert_eq!(editor.top, 4);
    editor.scroll(-9.0);
    assert_eq!(editor.top, 0);
}

#[test]
fn keeps_the_caret_in_sight_whichever_way_it_goes() {
    let start = Instant::now();
    let mut editor = Editor::new(
        "a.rs".to_owned(),
        "1\n2\n3\n4\n5\n6".to_owned(),
        &Touched::default(),
        Vec::new(),
        start,
    );
    editor.fit(TextWidth::new(20), 2);

    editor.buffer.place(4, 0, false);
    editor.settle(false);
    assert_eq!(editor.top, 3);
    editor.buffer.place(1, 0, false);
    editor.settle(false);
    assert_eq!(editor.top, 1);
    editor.fit(TextWidth::new(20), 0);
    assert_eq!(editor.visible, 1);
}

#[test]
fn stays_at_the_top_until_the_view_has_measured_its_rows() {
    let start = Instant::now();
    let mut editor = Editor::new(
        "a.rs".to_owned(),
        "1
2
3"
        .to_owned(),
        &Touched::default(),
        Vec::new(),
        start,
    );

    editor.buffer.place(2, 0, false);
    editor.settle(false);

    assert_eq!(editor.top, 0);
}

#[test]
fn appears_in_a_fade_then_stays_still() {
    let start = Instant::now();
    let editor = editor(start);
    let mut fading = Scene::default();
    let mut still = Scene::default();

    editor.fade_in(&mut fading, [100.0, 100.0], after(start, 100));
    editor.fade_in(&mut still, [100.0, 100.0], after(start, 450));

    assert!(fading.animating);
    assert!(editor.is_opening(after(start, 449)));
    assert!(!editor.is_opening(after(start, 450)));
    assert!(!still.animating);
}

#[test]
fn grows_to_its_size_around_the_middle_while_it_fades_in() {
    let start = Instant::now();
    let editor = editor(start);
    let mark = Rectangle {
        left: 300.0,
        top: 100.0,
        width: 1.0,
        height: 1.0,
        color: Rgba::ink(Ink::Full),
    };
    let mut scene = Scene {
        marks: vec![mark],
        ..Scene::default()
    };

    editor.fade_in(&mut scene, [100.0, 100.0], after(start, 100));

    let left = scene.marks[0].left;
    assert!(left > 100.0 + 200.0 * 0.97 && left < 300.0, "{left}");
}

#[test]
fn lets_the_wheel_take_the_view_away_from_the_caret() {
    let start = Instant::now();
    let mut editor = Editor::new(
        "a.rs".to_owned(),
        "1\n2\n3\n4\n5".to_owned(),
        &Touched::default(),
        Vec::new(),
        start,
    );
    editor.fit(TextWidth::new(20), 2);

    editor.scroll(3.0);
    editor.fit(TextWidth::new(20), 2);
    assert_eq!(editor.top, 3);
    editor.fit(TextWidth::new(20), 3);
    assert_eq!(editor.top, 0);
}
