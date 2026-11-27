use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::git::tests::ScratchFolder;

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn reads_an_ordinary_file_up_to_its_bound() {
    let folder = ScratchFolder::new("private-read");
    let path = folder.write("a", "abcdef");

    assert_eq!(read_at_most(&path, 4), Some(b"abcd".to_vec()));
    assert_eq!(read_at_most(&path, 6), Some(b"abcdef".to_vec()));
    assert_eq!(read_at_most(&path, 60), Some(b"abcdef".to_vec()));
}

#[test]
fn reads_nothing_that_is_not_an_ordinary_file() {
    let folder = ScratchFolder::new("private-not-a-file");

    assert_eq!(read_at_most(folder.path(), 10), None);
    assert_eq!(read_at_most(&folder.path().join("missing"), 10), None);
    assert!(open_ordinary(folder.path()).is_none());
}

#[test]
fn writes_a_whole_file_for_its_owner_only() {
    let parent = ScratchFolder::new("private-write");
    let folder = parent.path().join("deep/Dot");

    write_whole(&folder, "kept", |file| file.write_all(b"one")).unwrap();
    write_whole(&folder, "kept", |file| file.write_all(b"two")).unwrap();

    assert_eq!(fs::read(folder.join("kept")).unwrap(), b"two");
    assert_eq!(mode_of(&folder), PRIVATE_FOLDER);
    assert_eq!(mode_of(&folder.join("kept")), PRIVATE_FILE);
    assert!(!folder.join(format!("kept{BEING_WRITTEN}")).exists());
}

#[test]
fn a_write_that_fails_leaves_the_file_before_it_whole() {
    let folder = ScratchFolder::new("private-failed");
    write_whole(folder.path(), "kept", |file| file.write_all(b"before")).unwrap();

    let failed = write_whole(folder.path(), "kept", |file| {
        file.write_all(b"half")?;
        Err(io::Error::other("the disk is full"))
    });

    assert_eq!(failed.unwrap_err().to_string(), "the disk is full");
    assert_eq!(fs::read(folder.path().join("kept")).unwrap(), b"before");
}
