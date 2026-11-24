use std::fs::{self, OpenOptions};
use std::os::unix::fs::PermissionsExt;

use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::file::store::LARGEST_EDITED;
use crate::toolbox::tests::TempFolder;

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn changes(added: u64, removed: u64) -> ToolEnd {
    ToolEnd::Done(Detail::Changes { added, removed })
}

fn leftovers(folder: &TempFolder) -> Vec<String> {
    fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(".dot-"))
        .collect()
}

#[test]
fn creates_a_file_and_the_folders_above_it() {
    let folder = TempFolder::new("edit-create");
    let path = folder.join("src/deep/new.rs");

    let output = create(&path, "one\ntwo\nthree");

    assert_eq!(output.end, changes(3, 0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\nthree");
    assert!(text(&output).starts_with("Wrote 3 lines"));
}

#[test]
fn rewriting_a_file_counts_the_lines_it_replaced_and_keeps_its_mode() {
    let folder = TempFolder::new("edit-rewrite");
    let path = folder.write("run.sh", "#!/bin/sh\necho one\necho two\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();

    let output = create(&path, "#!/bin/sh\n");

    assert_eq!(output.end, changes(1, 3));
    assert_eq!(fs::read_to_string(&path).unwrap(), "#!/bin/sh\n");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o750
    );
    assert_eq!(leftovers(&folder).len(), 0);
}

#[test]
fn rewriting_keeps_windows_line_endings_and_a_new_file_keeps_its_own() {
    let folder = TempFolder::new("edit-crlf-create");
    let windows = folder.write("windows.txt", "a\r\nb\r\n");
    let fresh = folder.join("fresh.txt");

    create(&windows, "x\ny\n");
    create(&fresh, "p\r\nq\n");

    assert_eq!(fs::read_to_string(&windows).unwrap(), "x\r\ny\r\n");
    assert_eq!(fs::read_to_string(&fresh).unwrap(), "p\r\nq\n");
}

#[test]
fn will_not_write_over_a_folder() {
    let folder = TempFolder::new("edit-folder");
    folder.write("src/a.rs", "");

    let output = create(&folder.join("src"), "text");

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(text(&output).contains("is a folder"));
}

#[test]
fn replaces_the_only_occurrence() {
    let folder = TempFolder::new("edit-replace");
    let path = folder.write("a.rs", "fn one() {}\nfn two() {}\n");

    let output = replace(&path, "fn one() {}\n", "fn uno() {}\nfn dos() {}\n");

    assert_eq!(output.end, changes(2, 1));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "fn uno() {}\nfn dos() {}\nfn two() {}\n"
    );
    assert_eq!(leftovers(&folder).len(), 0);
}

#[test]
fn refuses_a_text_found_nowhere_or_more_than_once() {
    let folder = TempFolder::new("edit-replace-count");
    let path = folder.write("a.rs", "x = 1\nx = 1\ny = 2\n");

    let nowhere = replace(&path, "z = 3", "z = 4");
    let twice = replace(&path, "x = 1", "x = 2");
    let empty = replace(&path, "", "w");

    assert_eq!(nowhere.end, ToolEnd::Failed);
    assert!(text(&nowhere).contains("is not in"));
    assert_eq!(twice.end, ToolEnd::Failed);
    assert!(text(&twice).contains("2 times"));
    assert_eq!(empty.end, ToolEnd::Failed);
    assert!(text(&empty).contains("empty"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "x = 1\nx = 1\ny = 2\n");
}

#[test]
fn replaces_in_a_windows_file_what_claude_saw_without_carriage_returns() {
    let folder = TempFolder::new("edit-replace-crlf");
    let path = folder.write("a.txt", "one\r\ntwo\r\nthree\r\n");

    let output = replace(&path, "one\ntwo", "uno\ndos");

    assert_eq!(output.end, changes(2, 2));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "uno\r\ndos\r\nthree\r\n"
    );
}

#[test]
fn refuses_to_edit_what_is_not_utf8_text_or_too_large_or_missing() {
    let folder = TempFolder::new("edit-refused");
    let latin = folder.join("latin1.txt");
    fs::write(&latin, b"caf\xe9\n").unwrap();
    let huge = folder.write("huge.txt", "x\n");
    OpenOptions::new()
        .write(true)
        .open(&huge)
        .unwrap()
        .set_len(LARGEST_EDITED + 1)
        .unwrap();

    let not_utf8 = replace(&latin, "caf", "tea");
    let too_large = insert(&huge, 0, "y");
    let missing = replace(&folder.join("missing.txt"), "a", "b");
    let folder_itself = insert(folder.path(), 0, "y");

    assert!(text(&not_utf8).contains("not UTF-8"));
    assert!(text(&too_large).contains("larger than 64 MiB"));
    assert_eq!(missing.end, ToolEnd::Failed);
    assert!(text(&folder_itself).contains("is a folder"));
}

#[test]
fn a_file_of_exactly_the_largest_size_is_edited() {
    let folder = TempFolder::new("edit-largest");
    let path = folder.write("big.txt", "x\n");
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(LARGEST_EDITED)
        .unwrap();

    let output = insert(&path, 0, "first");

    assert_eq!(output.end, changes(1, 0));
}

#[test]
fn inserts_at_the_start_in_the_middle_and_at_the_end() {
    let folder = TempFolder::new("edit-insert");
    let path = folder.write("a.txt", "one\ntwo\n");

    insert(&path, 0, "zero\n");
    insert(&path, 2, "one and a half");
    let last = insert(&path, 4, "three\nfour");

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "zero\none\none and a half\ntwo\nthree\nfour\n"
    );
    assert_eq!(last.end, changes(2, 0));
}

#[test]
fn inserting_after_a_last_line_without_a_break_adds_the_break() {
    let folder = TempFolder::new("edit-insert-unfinished");
    let path = folder.write("a.txt", "one\ntwo");
    let empty = folder.write("empty.txt", "");

    insert(&path, 2, "three");
    insert(&empty, 0, "first");

    assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\nthree");
    assert_eq!(fs::read_to_string(&empty).unwrap(), "first\n");
}

#[test]
fn refuses_to_insert_past_the_end() {
    let folder = TempFolder::new("edit-insert-past");
    let path = folder.write("a.txt", "one\ntwo\n");

    let output = insert(&path, 3, "x");

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(text(&output).contains("has 2 lines"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\n");
}

#[test]
fn inserts_with_the_file_s_own_line_endings() {
    let folder = TempFolder::new("edit-insert-crlf");
    let path = folder.write("a.txt", "one\r\ntwo\r\n");

    let output = insert(&path, 1, "x\ny\n");

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "one\r\nx\r\ny\r\ntwo\r\n"
    );
    assert_eq!(output.end, changes(2, 0));
}

#[test]
fn inserting_nothing_adds_one_empty_line() {
    let folder = TempFolder::new("edit-insert-nothing");
    let path = folder.write("a.txt", "one\n");

    let output = insert(&path, 1, "");

    assert_eq!(fs::read_to_string(&path).unwrap(), "one\n\n");
    assert_eq!(output.end, changes(1, 0));
}

#[test]
fn trims_one_line_ending_only() {
    assert_eq!(trim_one_ending("a\r\n"), "a");
    assert_eq!(trim_one_ending("a\n\n"), "a\n");
    assert_eq!(trim_one_ending("a"), "a");
}

#[test]
fn finds_where_a_line_ends() {
    assert_eq!(offset_after("a\nb\nc", 0), 0);
    assert_eq!(offset_after("a\nb\nc", 1), 2);
    assert_eq!(offset_after("a\nb\nc", 2), 4);
    assert_eq!(offset_after("a\nb\nc", 3), 5);
}

#[test]
fn tells_what_each_edit_changed_line_by_line() {
    let folder = TempFolder::new("edit-change");
    let path = folder.join("a.rs");

    let created = create(&path, "a\nb\n");
    let replaced = replace(&path, "b", "B");
    let inserted = insert(&path, 1, "between");
    let rewritten = create(&path, "only\n");

    let created = created.change.unwrap();
    assert!(created.created);
    assert_eq!(
        created.hunks[0].lines,
        [
            dot_api::PatchLine::Added("a".to_owned()),
            dot_api::PatchLine::Added("b".to_owned())
        ]
    );
    let replaced = replaced.change.unwrap();
    assert!(!replaced.created);
    assert!(
        replaced.hunks[0]
            .lines
            .contains(&dot_api::PatchLine::Removed("b".to_owned()))
    );
    assert!(
        replaced.hunks[0]
            .lines
            .contains(&dot_api::PatchLine::Added("B".to_owned()))
    );
    let inserted = inserted.change.unwrap();
    assert!(
        inserted.hunks[0]
            .lines
            .contains(&dot_api::PatchLine::Added("between".to_owned()))
    );
    assert!(!rewritten.change.unwrap().created);
}

#[test]
fn a_failed_edit_tells_no_change() {
    let folder = TempFolder::new("edit-no-change");

    let missing = replace(&folder.join("none.rs"), "a", "b");

    assert_eq!(missing.change, None);
}

#[test]
fn borrows_a_text_whose_line_endings_already_fit() {
    assert!(matches!(in_endings("a\nb", false), Cow::Borrowed("a\nb")));
    assert_eq!(in_endings("a\r\nb", false), "a\nb");
    assert_eq!(in_endings("a\nb", true), "a\r\nb");
    assert_eq!(in_endings("a\r\nb\n", true), "a\r\nb\r\n");
}

#[test]
fn turns_bytes_into_text_without_losing_a_line() {
    assert_eq!(into_text(b"a\nb".to_vec()), "a\nb");
    assert_eq!(into_text(vec![b'a', 0xff, b'\n', b'b']), "a\u{fffd}\nb");
}
