use std::path::PathBuf;

use serde_json::json;

use super::*;

struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("dot-change-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&path));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn with(self, entries: &[&str]) -> Self {
        for entry in entries {
            match entry.strip_suffix('/') {
                Some(folder) => fs::create_dir_all(self.0.join(folder)).unwrap(),
                None => fs::write(self.0.join(entry), "").unwrap(),
            }
        }
        self
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn created(path: &str) -> FileChange {
    FileChange::between(path, "", "a\n", true)
}

#[test]
fn names_a_file_inside_the_folder_from_the_folder() {
    let folder = Folder::new("relative");
    let path = folder.0.join("src/a.rs");
    let change = FileChange::between(path.to_str().unwrap(), "a\n", "b\n", false);

    let shown = as_shown(change, Some(&folder.0));

    assert_eq!(shown.path, "src/a.rs");
    assert_eq!(shown.siblings, None);
}

#[test]
fn places_a_created_file_between_its_two_neighbours_on_each_side() {
    let folder = Folder::new("siblings").with(&[
        "src/",
        "src/build.rs",
        "src/Cargo.rs",
        "src/lib.rs",
        "src/new.rs",
        "src/old.rs",
        "src/tests/",
        "src/zeta.rs",
        "src/.hidden",
    ]);

    let shown = as_shown(created("src/new.rs"), Some(&folder.0));

    assert_eq!(
        shown.siblings,
        Some(Siblings {
            folder: "src/".to_owned(),
            before: vec!["Cargo.rs".to_owned(), "lib.rs".to_owned()],
            after: vec!["old.rs".to_owned(), "tests/".to_owned()],
        })
    );
    assert_eq!(shown.path, "src/new.rs");
}

#[test]
fn has_fewer_neighbours_at_the_ends_of_a_listing() {
    let folder = Folder::new("ends").with(&["a.rs", "b.rs"]);
    let path = folder.0.join("0.rs");

    let shown = as_shown(created(path.to_str().unwrap()), None);

    let siblings = shown.siblings.unwrap();
    assert_eq!(siblings.before, Vec::<String>::new());
    assert_eq!(siblings.after, ["a.rs", "b.rs"]);
    assert!(siblings.folder.ends_with('/'));
}

#[test]
fn tells_no_neighbours_of_a_file_whose_folder_cannot_be_read() {
    let shown = as_shown(created("/dot-missing-folder/new.rs"), None);

    assert_eq!(shown.siblings, None);
}

#[test]
fn reads_an_edit_of_claude_code_and_widens_it_with_the_file_before() {
    let result = json!({
        "filePath": "/p/keep.txt",
        "oldString": "f",
        "newString": "F",
        "originalFile": "a\nb\nc\nd\ne\nf\ng\nh\n",
        "structuredPatch": [{"oldStart": 3, "oldLines": 6, "newStart": 3, "newLines": 6,
            "lines": [" c", " d", " e", "-f", "+F", " g", " h"]}],
        "userModified": false,
        "replaceAll": false
    });

    let change = of_claude_code(&result).unwrap();

    assert_eq!(change.path, "/p/keep.txt");
    assert!(!change.created);
    let hunk = &change.hunks[0];
    assert_eq!((hunk.old_start, hunk.new_start), (1, 1));
    assert_eq!(hunk.lines[0], PatchLine::Same("a".to_owned()));
    assert_eq!(hunk.lines[5], PatchLine::Removed("f".to_owned()));
    assert_eq!(hunk.lines[6], PatchLine::Added("F".to_owned()));
    assert_eq!(hunk.lines.len(), 9);
}

#[test]
fn keeps_an_edit_as_it_came_without_the_file_before() {
    let result = json!({
        "filePath": "/p/a.rs",
        "structuredPatch": [{"oldStart": 7, "newStart": 7, "lines": [" x", "-y", "+Y", "?odd", ""]}]
    });

    let change = of_claude_code(&result).unwrap();

    assert_eq!(change.hunks[0].old_start, 7);
    assert_eq!(change.hunks[0].lines.len(), 3);
}

#[test]
fn reads_a_file_claude_code_created() {
    let result = json!({
        "type": "create",
        "filePath": "/p/new.txt",
        "content": "one\ntwo\n",
        "structuredPatch": [],
        "originalFile": null
    });

    let change = of_claude_code(&result).unwrap();

    assert!(change.created);
    assert_eq!(
        change.hunks[0].lines,
        [
            PatchLine::Added("one".to_owned()),
            PatchLine::Added("two".to_owned())
        ]
    );
}

#[test]
fn reads_no_change_where_there_is_none() {
    let read =
        json!({"type": "text", "file": {"filePath": "/p/a.rs", "content": "x", "numLines": 1}});
    let search = json!({"filePath": "/p/a.rs", "content": "x", "numLines": 1});
    let update =
        json!({"type": "update", "filePath": "/p/a.rs", "content": "x", "structuredPatch": []});
    let nameless = json!({"type": "create", "content": "x"});

    assert_eq!(of_claude_code(&read), None);
    assert_eq!(of_claude_code(&search), None);
    assert_eq!(of_claude_code(&update), None);
    assert_eq!(of_claude_code(&nameless), None);
    assert_eq!(of_claude_code(&json!("Error: refused")), None);
}

#[test]
fn reads_each_kind_of_line_of_a_patch() {
    assert_eq!(line_of(" a"), Some(PatchLine::Same("a".to_owned())));
    assert_eq!(line_of("-é"), Some(PatchLine::Removed("é".to_owned())));
    assert_eq!(line_of("+"), Some(PatchLine::Added(String::new())));
    assert_eq!(line_of("\\ No newline at end of file"), None);
    assert_eq!(line_of(""), None);
}
