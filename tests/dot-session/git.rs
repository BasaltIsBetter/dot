use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::Command;

use proptest::prelude::*;

use super::*;

const MAIN_HEAD: &str = "ref: refs/heads/feat/mac-app\n";
const COMMIT: &str = "53964a7c0ffee1234567890abcdef0123456789a";

pub(crate) struct ScratchFolder {
    root: PathBuf,
}

impl ScratchFolder {
    pub(crate) fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("dot-session-{}-{name}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Self {
            root: fs::canonicalize(root).unwrap(),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.root
    }

    pub(crate) fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for ScratchFolder {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("could not remove {}: {error}", self.root.display());
        }
    }
}

const MAIN: Option<&str> = Some("main");

fn branch_of(name: &str, head: &str) -> Option<String> {
    let folder = ScratchFolder::new(name);
    folder.write(HEAD_PATH, head);
    branch(folder.path())
}

fn main_branch() -> Option<String> {
    MAIN.map(str::to_owned)
}

#[test]
fn reads_the_checked_out_branch() {
    assert_eq!(
        branch_of("named", MAIN_HEAD),
        Some("feat/mac-app".to_owned())
    );
}

#[test]
fn shows_a_short_hash_when_no_branch_is_checked_out() {
    assert_eq!(branch_of("detached", COMMIT), Some("53964a7".to_owned()));
    assert_eq!(
        branch_of("detached-sha256", &COMMIT.repeat(2)[..64]),
        Some("53964a7".to_owned())
    );
}

#[test]
fn keeps_any_other_reference_whole() {
    assert_eq!(
        branch_of("remote", "ref: refs/remotes/origin/main"),
        Some("refs/remotes/origin/main".to_owned())
    );
}

#[test]
fn a_folder_without_git_has_no_branch() {
    let folder = ScratchFolder::new("no-git");

    assert_eq!(branch(folder.path()), None);
}

#[test]
fn shows_main_for_a_repository_whose_head_git_would_not_write() {
    assert_eq!(branch_of("garbage", "hello"), main_branch());
    assert_eq!(branch_of("empty", ""), main_branch());
    assert_eq!(branch_of("empty-reference", "ref: "), main_branch());
    assert_eq!(branch_of("short-hash", &COMMIT[..12]), main_branch());
    assert_eq!(branch_of("not-hex", &"g".repeat(40)), main_branch());
}

#[test]
fn shows_main_for_a_repository_without_head() {
    let folder = ScratchFolder::new("no-head");
    fs::create_dir_all(folder.path().join(".git")).unwrap();

    assert_eq!(branch(folder.path()), main_branch());
}

#[test]
fn rejects_a_branch_with_control_characters() {
    assert_eq!(
        branch_of("control", "ref: refs/heads/evil\u{1b}[31mred"),
        main_branch()
    );
}

#[test]
fn rejects_a_branch_that_would_reverse_the_bar() {
    assert_eq!(
        branch_of("reversed", "ref: refs/heads/evil\u{202e}live"),
        main_branch()
    );
}

#[test]
fn never_follows_git_out_of_the_folder() {
    let outside = ScratchFolder::new("outside");
    outside.write(HEAD_PATH, MAIN_HEAD);
    let linked_git = ScratchFolder::new("linked-git");
    symlink(outside.path().join(".git"), linked_git.path().join(".git")).unwrap();
    let linked_head = ScratchFolder::new("linked-head");
    fs::create_dir_all(linked_head.path().join(".git")).unwrap();
    symlink(
        outside.path().join(HEAD_PATH),
        linked_head.path().join(HEAD_PATH),
    )
    .unwrap();

    assert_eq!(branch(outside.path()), Some("feat/mac-app".to_owned()));
    assert_eq!(branch(linked_git.path()), main_branch());
    assert_eq!(branch(linked_head.path()), main_branch());
}

#[test]
fn shows_main_for_a_worktree_without_following_its_git_file() {
    let folder = ScratchFolder::new("worktree");
    folder.write(".git", "gitdir: /elsewhere/.git/worktrees/dot\n");

    assert_eq!(branch(folder.path()), main_branch());
}

#[test]
fn never_opens_a_head_that_is_not_a_regular_file() {
    let folder = ScratchFolder::new("fifo");
    fs::create_dir_all(folder.path().join(".git")).unwrap();
    let made = Command::new("mkfifo")
        .arg(folder.path().join(HEAD_PATH))
        .status()
        .unwrap();
    assert!(made.success());

    assert_eq!(branch(folder.path()), main_branch());
}

#[test]
fn reads_no_further_than_the_start_of_head() {
    let long_name = "a".repeat(2_000);

    let read = branch_of("long", &format!("ref: refs/heads/{long_name}")).unwrap();

    assert_eq!(read.len(), 512 - BRANCH_REFERENCE.len());
}

proptest! {
    #[test]
    fn shows_a_branch_for_any_head_of_a_repository(contents in "\\PC{0,80}") {
        let folder = ScratchFolder::new("any-head");
        folder.write(HEAD_PATH, &contents);

        let shown = branch(folder.path());

        prop_assert_eq!(shown, Some(parse_head(&contents).unwrap_or_else(|| DEFAULT_BRANCH.to_owned())));
    }

    #[test]
    fn parses_any_text_into_a_readable_name_or_nothing(contents in "\\PC{0,80}|[\\x00-\\x7f]{0,80}") {
        if let Some(name) = parse_head(&contents) {
            prop_assert!(!name.is_empty());
            prop_assert!(name.chars().all(crate::readable::is_readable));
            prop_assert!(contents.contains(&name));
        }
    }
}
