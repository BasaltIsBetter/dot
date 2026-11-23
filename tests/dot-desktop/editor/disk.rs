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
