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
