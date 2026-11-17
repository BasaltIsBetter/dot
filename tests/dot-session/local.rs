use std::os::unix::fs::symlink;

use super::*;
use crate::git::tests::ScratchFolder;

const HEAD_PATH: &str = ".git/HEAD";

fn session_in(folder: &Path) -> LocalSession {
    LocalSession::new(Model::OPUS_5_5, Some(folder.to_path_buf()))
}

fn name_of(folder: &ScratchFolder) -> String {
    folder
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn reports_the_folder_and_its_branch() {
    let folder = ScratchFolder::new("project");
    folder.write(HEAD_PATH, "ref: refs/heads/feat/mac-app\n");

    let status = session_in(folder.path()).status();

    assert_eq!(status.project, Some(name_of(&folder)));
    assert_eq!(status.branch, Some("feat/mac-app".to_owned()));
}

#[test]
fn talks_to_the_model_it_was_given() {
    let status = LocalSession::new(Model::OPUS_5_5, None).status();

    assert_eq!(status.model, Model::OPUS_5_5);
}

#[test]
fn starts_with_nothing_consumed() {
    let folder = ScratchFolder::new("fresh");

    assert_eq!(session_in(folder.path()).status().usage, Usage::default());
}

#[test]
fn a_session_without_folder_has_no_project() {
    let status = LocalSession::new(Model::OPUS_5_5, None).status();

    assert_eq!((status.project, status.branch), (None, None));
}

#[test]
fn a_missing_folder_or_a_file_is_no_project() {
    let folder = ScratchFolder::new("not-a-folder");
    let file = folder.write("notes.txt", "");

    let missing = session_in(&folder.path().join("missing")).status();
    let not_a_folder = session_in(&file).status();

    assert_eq!((missing.project, missing.branch), (None, None));
    assert_eq!((not_a_folder.project, not_a_folder.branch), (None, None));
}

#[test]
fn names_a_linked_folder_after_the_real_one() {
    let real = ScratchFolder::new("real");
    let links = ScratchFolder::new("links");
    let alias = links.path().join("alias");
    symlink(real.path(), &alias).unwrap();

    let status = session_in(&alias).status();

    assert_eq!(status.project, Some(name_of(&real)));
}

#[test]
fn replaces_unreadable_characters_in_the_folder_name() {
    let parent = ScratchFolder::new("unreadable");
    let folder = parent.path().join("line\nbreak");
    fs::create_dir(&folder).unwrap();

    let status = session_in(&folder).status();

    assert_eq!(status.project, Some("line\u{fffd}break".to_owned()));
}

#[test]
fn replaces_invisible_reordering_characters_in_the_folder_name() {
    let parent = ScratchFolder::new("reordering");
    let folder = parent.path().join("dot\u{202e}tod");
    fs::create_dir(&folder).unwrap();

    let status = session_in(&folder).status();

    assert_eq!(status.project, Some("dot\u{fffd}tod".to_owned()));
}

#[test]
fn reads_the_branch_again_at_each_status() {
    let folder = ScratchFolder::new("checkout");
    folder.write(HEAD_PATH, "ref: refs/heads/main\n");
    let session = session_in(folder.path());
    let before = session.status();

    folder.write(HEAD_PATH, "ref: refs/heads/feat/status-bar\n");

    assert_eq!(before.branch, Some("main".to_owned()));
    assert_eq!(session.status().branch, Some("feat/status-bar".to_owned()));
}
