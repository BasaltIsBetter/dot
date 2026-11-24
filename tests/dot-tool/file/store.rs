use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::toolbox::tests::TempFolder;

#[test]
fn a_temporary_name_already_taken_is_skipped() {
    let folder = TempFolder::new("edit-temporary");
    let path = folder.write("a.txt", "one\n");
    let taken = folder.join(&format!(".a.txt.dot-{}-0.tmp", std::process::id()));
    fs::write(&taken, "someone else's").unwrap();

    write_atomically(&path, b"two\n", None).unwrap();

    assert_eq!(fs::read_to_string(&taken).unwrap(), "someone else's");
    assert_eq!(fs::read_to_string(&path).unwrap(), "two\n");
}

#[test]
fn writes_with_the_permissions_it_is_given() {
    let folder = TempFolder::new("store-mode");
    let path = folder.join("tool.sh");

    write_atomically(
        &path,
        b"#!/bin/sh\n",
        Some(fs::Permissions::from_mode(0o700)),
    )
    .unwrap();

    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[test]
fn a_missing_file_has_no_original_but_a_folder_is_refused() {
    let folder = TempFolder::new("store-original");

    assert!(
        read_original(&folder.join("missing.txt"))
            .unwrap()
            .is_none()
    );
    assert!(read_original(folder.path()).is_err());
}

#[test]
fn leaves_no_temporary_file_when_the_rename_fails() {
    let folder = TempFolder::new("store-rename");
    folder.write("taken/inside.txt", "");

    let written = write_atomically(&folder.join("taken"), b"x", None);

    assert!(written.is_err());
    let leftovers: Vec<_> = fs::read_dir(folder.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".dot-"))
        .collect();
    assert_eq!(leftovers.len(), 0);
}

#[test]
fn refuses_to_wait_on_a_named_pipe() {
    let folder = crate::toolbox::tests::TempFolder::new("store-pipe");
    let pipe = folder.pipe("fifo");
    let shown = pipe.clone();

    let refused = crate::toolbox::tests::without_waiting(move || {
        (
            read_original(&pipe).err(),
            read_text(&pipe).err().map(|error| error.len()),
        )
    });

    assert_eq!(refused.0, Some(not_a_file(&shown)));
    assert!(refused.1.is_some());
}

#[test]
fn the_copy_that_replaces_a_file_is_its_owner_s_alone_while_it_is_written() {
    let folder = TempFolder::new("store-private");
    let path = folder.write(".env", "SECRET=1\n");

    let (temporary, file) = temporary_beside(&path, true).unwrap();

    assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    fs::remove_file(temporary).unwrap();
}

#[test]
fn a_new_file_takes_the_permissions_any_new_file_takes() {
    let folder = TempFolder::new("store-new");
    let made = folder.write("made.txt", "");
    let path = folder.join("new.txt");

    write_atomically(&path, b"new\n", None).unwrap();

    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode(),
        fs::metadata(&made).unwrap().permissions().mode()
    );
}

#[test]
fn names_the_copy_of_a_long_name_within_what_macos_allows() {
    for name in ["x".repeat(300), "é".repeat(200), "a.txt".to_owned()] {
        let temporary = temporary_name(&name, 63);

        assert!(temporary.len() <= 255, "{}", temporary.len());
        let first = name.chars().next().unwrap();
        assert!(temporary.starts_with(&format!(".{first}")));
        assert!(temporary.ends_with("-63.tmp"));
    }
    assert_eq!(
        temporary_name("a.txt", 0),
        format!(".a.txt.dot-{}-0.tmp", std::process::id())
    );
}

#[test]
fn writes_a_file_whose_name_is_as_long_as_macos_allows() {
    let folder = TempFolder::new("store-long-name");
    let path = folder.join(&"n".repeat(250));

    write_atomically(&path, b"long\n", Some(fs::Permissions::from_mode(0o644))).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "long\n");
}

#[test]
fn reads_the_first_line_of_a_file_and_its_permissions_only() {
    let folder = TempFolder::new("store-first-line");
    let path = folder.write("a.txt", "one\r\ntwo\r\nthree\r\n");
    let one_line = folder.write("b.txt", "only");

    let first = read_first_line(&path).unwrap().unwrap();
    let only = read_first_line(&one_line).unwrap().unwrap();

    assert_eq!(first.bytes, b"one\r\n");
    assert!(first.permissions.is_some());
    assert_eq!(only.bytes, b"only");
    assert!(
        read_first_line(&folder.join("missing.txt"))
            .unwrap()
            .is_none()
    );
    assert!(read_first_line(folder.path()).is_err());
}

#[test]
fn reading_the_first_line_never_waits_on_a_named_pipe() {
    let folder = TempFolder::new("store-first-pipe");
    let pipe = folder.pipe("fifo");
    let shown = pipe.clone();

    let refused = crate::toolbox::tests::without_waiting(move || read_first_line(&pipe).err());

    assert_eq!(refused, Some(not_a_file(&shown)));
}

#[test]
fn reads_no_more_of_a_first_line_than_the_largest_file_the_editor_takes_and_a_byte() {
    let largest = usize::try_from(LARGEST_EDITED).unwrap();
    let folder = TempFolder::new("store-first-line-long");
    let path = folder.write("long.txt", &"x".repeat(largest + 10));

    let first = read_first_line(&path).unwrap().unwrap();

    assert_eq!(first.bytes.len(), largest + 1);
}

#[test]
fn says_why_a_first_line_cannot_be_read_when_the_file_is_not_merely_missing() {
    let folder = TempFolder::new("store-first-line-under-a-file");
    folder.write("a.txt", "one");

    assert!(read_first_line(&folder.join("a.txt/inside")).is_err());
}
