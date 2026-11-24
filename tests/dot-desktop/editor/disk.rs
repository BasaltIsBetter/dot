use std::fs;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use super::*;
use crate::editor::tests::MemoryWorkspace;

const WAIT: Duration = Duration::from_secs(5);

fn disk(
    workspace: MemoryWorkspace,
) -> (
    Disk<MemoryWorkspace, mpsc::Sender<DiskNews>>,
    Receiver<DiskNews>,
    Arc<MemoryWorkspace>,
) {
    let (sender, receiver) = mpsc::channel();
    let workspace = Arc::new(workspace);
    (
        Disk::new(Arc::clone(&workspace), sender),
        receiver,
        workspace,
    )
}

#[test]
fn opens_a_file_with_the_listing_of_its_folder() {
    let (mut disk, news, _) = disk(MemoryWorkspace::with(&[
        ("src/a.rs", "a"),
        ("src/b.rs", ""),
        ("top.rs", ""),
    ]));

    disk.open("src/a.rs".to_owned());

    assert_eq!(
        news.recv_timeout(WAIT).unwrap(),
        DiskNews::Opened {
            path: "src/a.rs".to_owned(),
            text: Ok("a".to_owned()),
            entries: vec!["a.rs".to_owned(), "b.rs".to_owned()],
        }
    );
}

#[test]
fn tells_why_a_file_would_not_open() {
    let (mut disk, news, _) = disk(MemoryWorkspace::default());

    disk.open("none.rs".to_owned());

    assert_eq!(
        news.recv_timeout(WAIT).unwrap(),
        DiskNews::Opened {
            path: "none.rs".to_owned(),
            text: Err(WorkspaceError::NotAFile),
            entries: Vec::new(),
        }
    );
}

#[test]
fn saves_each_keystroke_and_ends_on_the_last_text() {
    let (mut disk, news, workspace) = disk(MemoryWorkspace::with(&[("a.rs", "")]));

    for text in ["a", "ab", "abc"] {
        disk.save("a.rs", text);
    }
    drop(disk);

    assert_eq!(workspace.text("a.rs").as_deref(), Some("abc"));
    let saved: Vec<DiskNews> = news.try_iter().collect();
    assert!(!saved.is_empty() && saved.len() <= 3);
    assert!(saved.iter().all(|news| *news
        == DiskNews::Saved {
            path: "a.rs".to_owned(),
            result: Ok(())
        }));
}

#[test]
fn writes_every_file_of_a_burst_even_when_they_alternate() {
    let (mut disk, _news, workspace) = disk(MemoryWorkspace::default());

    disk.save("a.rs", "a1");
    disk.save("b.rs", "b1");
    disk.save("a.rs", "a2");
    drop(disk);

    assert_eq!(workspace.text("a.rs").as_deref(), Some("a2"));
    assert_eq!(workspace.text("b.rs").as_deref(), Some("b1"));
}

#[test]
fn tells_when_a_keystroke_could_not_be_saved() {
    let (mut disk, news, _) = disk(MemoryWorkspace::default().refusing());

    disk.save("a.rs", "a");

    assert_eq!(
        news.recv_timeout(WAIT).unwrap(),
        DiskNews::Saved {
            path: "a.rs".to_owned(),
            result: Err(WorkspaceError::Failed("disk full".to_owned())),
        }
    );
}

#[test]
fn names_the_folder_of_a_path() {
    assert_eq!(folder_of("crates/a/src/b.rs"), "crates/a/src");
    assert_eq!(folder_of("b.rs"), "");
}

#[test]
fn keeps_only_the_last_text_waiting_for_each_file() {
    let mut latest = Vec::new();

    keep_latest(&mut latest, "a.rs", "a1");
    keep_latest(&mut latest, "b.rs", "b1");
    keep_latest(&mut latest, "a.rs", "a2");

    assert_eq!(
        latest,
        [
            ("a.rs".to_owned(), "a2".to_owned()),
            ("b.rs".to_owned(), "b1".to_owned())
        ]
    );
}

#[test]
fn hands_the_writer_what_waits_then_nothing_once_finished() {
    let waiting = Waiting::default();
    keep_latest(&mut waiting.pending().latest, "a.rs", "a");

    let first = waiting.next();
    waiting.pending().finished = true;
    let after = waiting.next();

    assert_eq!(first, Some(vec![("a.rs".to_owned(), "a".to_owned())]));
    assert_eq!(after, None);
}

#[test]
fn hands_the_writer_what_still_waits_when_it_is_finished() {
    let waiting = Waiting::default();
    keep_latest(&mut waiting.pending().latest, "a.rs", "a");
    waiting.pending().finished = true;

    assert_eq!(
        waiting.next(),
        Some(vec![("a.rs".to_owned(), "a".to_owned())])
    );
    assert_eq!(waiting.next(), None);
}

#[test]
fn finishing_writes_what_waits_and_lets_the_writer_go() {
    let (mut disk, _news, workspace) = disk(MemoryWorkspace::with(&[("a.rs", "")]));
    disk.save("a.rs", "a");
    disk.save("a.rs", "ab");

    disk.finish();

    assert_eq!(workspace.text("a.rs").as_deref(), Some("ab"));
    assert!(disk.saver.is_none());
}

struct Panicking;

impl Workspace for Panicking {
    fn read(&self, _path: &str) -> Result<String, WorkspaceError> {
        panic!("the workspace fails to read");
    }

    fn write(&self, _path: &str, _text: &str) -> Result<(), WorkspaceError> {
        Ok(())
    }

    fn list(&self, _folder: &str) -> Result<Vec<String>, WorkspaceError> {
        Ok(Vec::new())
    }
}

#[test]
fn tells_that_a_file_could_not_be_read_when_reading_it_crashed() {
    let (sender, news) = mpsc::channel::<DiskNews>();
    let mut disk = Disk::new(Arc::new(Panicking), sender);

    disk.open("a.rs".to_owned());

    assert_eq!(
        news.recv_timeout(WAIT).unwrap(),
        DiskNews::Opened {
            path: "a.rs".to_owned(),
            text: Err(WorkspaceError::Failed(READER_FELL.to_owned())),
            entries: Vec::new(),
        }
    );
}

#[test]
fn reads_no_image_from_what_is_not_a_file() {
    let folder = std::env::temp_dir().join(format!("dot-disk-drop-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    fs::create_dir_all(&folder).unwrap();
    let pipe = folder.join("pipe.png");
    let made = std::process::Command::new("/usr/bin/mkfifo")
        .arg(&pipe)
        .status()
        .unwrap();
    assert!(made.success());

    assert_eq!(read_image(&pipe), Err(ImageError::Unsupported));
    assert_eq!(read_image(&folder), Err(ImageError::Unsupported));
    assert_eq!(
        read_image(Path::new("/nonexistent/capture.png")),
        Err(ImageError::Unsupported)
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn reads_the_bytes_of_an_image_file_and_one_more_than_an_image_may_have() {
    let folder = std::env::temp_dir().join(format!("dot-disk-image-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    fs::create_dir_all(&folder).unwrap();
    let small = folder.join("small.png");
    let huge = folder.join("huge.png");
    fs::write(&small, b"\x89PNG\r\n\x1a\n pixels").unwrap();
    fs::write(&huge, vec![0; Image::LARGEST + 10]).unwrap();

    assert_eq!(read_image(&small).unwrap(), b"\x89PNG\r\n\x1a\n pixels");
    assert_eq!(read_image(&huge).unwrap().len(), Image::LARGEST + 1);
    fs::remove_dir_all(&folder).unwrap();
}
