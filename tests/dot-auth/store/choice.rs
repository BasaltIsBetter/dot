use std::path::Path;

use dot_api::ApiKey;

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

    assert_eq!(choice_in(&folder).choice().unwrap(), None);
}

#[test]
fn remembers_claude_code_in_a_folder_it_creates() {
    let folder = ScratchFolder::new("remember");
    let choice = choice_in(&folder);

    choice.remember(&Remembered::ClaudeCode).unwrap();

    assert_eq!(choice.choice().unwrap(), Some(Remembered::ClaudeCode));
    assert_eq!(
        fs::read_to_string(folder.path().join("Dot/connection")).unwrap(),
        "claude-code"
    );
}

#[test]
fn forgets_the_choice_and_tolerates_forgetting_twice() {
    let folder = ScratchFolder::new("forget");
    let choice = choice_in(&folder);
    choice.remember(&Remembered::ClaudeCode).unwrap();

    choice.forget().unwrap();

    assert_eq!(choice.choice().unwrap(), None);
    assert!(choice.forget().is_ok());
}

#[test]
fn ignores_a_file_that_says_anything_else() {
    let folder = ScratchFolder::new("other");
    fs::create_dir_all(folder.path().join("Dot")).unwrap();
    fs::write(folder.path().join("Dot/connection"), "claude-code\nextra").unwrap();

    assert_eq!(choice_in(&folder).choice().unwrap(), None);
}

#[test]
fn reports_a_choice_it_cannot_read() {
    let folder = ScratchFolder::new("unreadable");
    fs::create_dir_all(folder.path().join("Dot/connection")).unwrap();

    let error = choice_in(&folder).choice().unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::StorageFailed);
}

#[test]
fn remembers_an_anthropic_account_by_its_name() {
    let folder = ScratchFolder::new("account");
    let choice = choice_in(&folder);

    choice.remember(&Remembered::AnthropicAccount).unwrap();

    assert_eq!(choice.choice().unwrap(), Some(Remembered::AnthropicAccount));
    assert_eq!(
        fs::read_to_string(folder.path().join("Dot/connection")).unwrap(),
        "anthropic-account"
    );
}

#[test]
fn never_writes_an_api_key_and_forgets_instead() {
    let folder = ScratchFolder::new("never-key");
    let choice = choice_in(&folder);
    choice.remember(&Remembered::ClaudeCode).unwrap();
    let key = ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap();

    choice.remember(&Remembered::ApiKey(key)).unwrap();

    assert_eq!(choice.choice().unwrap(), None);
    assert!(!folder.path().join("Dot/connection").exists());
}

#[test]
fn reports_a_choice_it_cannot_open() {
    let folder = ScratchFolder::new("unopenable");
    fs::write(folder.path().join("Dot"), "a file where a folder should be").unwrap();

    let error = choice_in(&folder).choice().unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::StorageFailed);
}
