use std::path::Path;

use super::*;

pub(crate) struct ScratchFolder {
    root: PathBuf,
}

impl ScratchFolder {
    pub(crate) fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("dot-auth-{}-{name}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for ScratchFolder {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("could not remove {}: {error}", self.root.display());
        }
    }
}

pub(crate) fn choice_in(folder: &ScratchFolder) -> ChoiceFile {
    ChoiceFile::new(folder.path().join("Dot/connection"))
}

#[test]
fn holds_nothing_before_anything_is_written() {
    let folder = ScratchFolder::new("empty");

    assert!(!choice_in(&folder).holds_claude_code().unwrap());
}

#[test]
fn remembers_claude_code_in_a_folder_it_creates() {
    let folder = ScratchFolder::new("remember");
    let choice = choice_in(&folder);

    choice.remember_claude_code().unwrap();

    assert!(choice.holds_claude_code().unwrap());
    assert_eq!(
        fs::read_to_string(folder.path().join("Dot/connection")).unwrap(),
        "claude-code"
    );
}

#[test]
fn forgets_the_choice_and_tolerates_forgetting_twice() {
    let folder = ScratchFolder::new("forget");
    let choice = choice_in(&folder);
    choice.remember_claude_code().unwrap();

    choice.forget().unwrap();

    assert!(!choice.holds_claude_code().unwrap());
    assert!(choice.forget().is_ok());
}

#[test]
fn ignores_a_file_that_says_anything_else() {
    let folder = ScratchFolder::new("other");
    fs::create_dir_all(folder.path().join("Dot")).unwrap();
    fs::write(folder.path().join("Dot/connection"), "claude-code\nextra").unwrap();

    assert!(!choice_in(&folder).holds_claude_code().unwrap());
}

#[test]
fn reports_a_choice_it_cannot_read() {
    let folder = ScratchFolder::new("unreadable");
    fs::create_dir_all(folder.path().join("Dot/connection")).unwrap();

    let error = choice_in(&folder).holds_claude_code().unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::StorageFailed);
}
