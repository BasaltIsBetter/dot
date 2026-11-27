use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::git::tests::ScratchFolder;

fn prompts(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

#[test]
fn keeps_the_prompts_for_the_next_launch() {
    let folder = ScratchFolder::new("prompts-kept");
    let history = FilePromptHistory::new(Some(folder.path().to_path_buf()));

    history.save(&prompts(&["first", "second\nline"])).unwrap();

    assert_eq!(
        FilePromptHistory::new(Some(folder.path().to_path_buf())).load(),
        prompts(&["first", "second\nline"])
    );
    let mode = fs::metadata(folder.path().join(FILE_NAME))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn writes_stable_names() {
    let folder = ScratchFolder::new("prompts-names");

    FilePromptHistory::new(Some(folder.path().to_path_buf()))
        .save(&prompts(&["a"]))
        .unwrap();

    assert_eq!(
        fs::read_to_string(folder.path().join(FILE_NAME)).unwrap(),
        r#"{"format":1,"prompts":["a"]}"#
    );
}

#[test]
fn reads_nothing_that_is_not_its_own() {
    let folder = ScratchFolder::new("prompts-other");
    let history = FilePromptHistory::new(Some(folder.path().to_path_buf()));

    assert_eq!(history.load(), Vec::<String>::new());
    for written in [
        r#"{"format":2,"prompts":["a"]}"#,
        r#"{"prompts":["a"]}"#,
        "not json",
    ] {
        folder.write(FILE_NAME, written);
        assert_eq!(history.load(), Vec::<String>::new(), "{written}");
    }
}

#[test]
fn leaves_out_what_is_not_a_prompt() {
    let folder = ScratchFolder::new("prompts-mixed");
    folder.write(FILE_NAME, r#"{"format":1,"prompts":["a",7,"",null,"b"]}"#);

    assert_eq!(
        FilePromptHistory::new(Some(folder.path().to_path_buf())).load(),
        prompts(&["a", "b"])
    );
}

#[test]
fn reads_nothing_of_a_file_too_large() {
    let folder = ScratchFolder::new("prompts-large");
    let padding = " ".repeat(usize::try_from(LARGEST_FILE).unwrap());
    folder.write(
        FILE_NAME,
        &format!(r#"{{"format":1,"prompts":["a"]}}{padding}"#),
    );

    assert_eq!(
        FilePromptHistory::new(Some(folder.path().to_path_buf())).load(),
        Vec::<String>::new()
    );
}

#[test]
fn keeps_nothing_without_a_folder() {
    let history = FilePromptHistory::new(None);

    assert_eq!(history.load(), Vec::<String>::new());
    assert_eq!(
        history.save(&prompts(&["a"])),
        Err(ArchiveError(NO_FOLDER.to_owned()))
    );
}

#[test]
fn reads_a_file_of_its_largest_size() {
    let folder = ScratchFolder::new("prompts-largest");
    let kept = r#"{"format":1,"prompts":["a"]}"#;
    let padding = " ".repeat(usize::try_from(LARGEST_FILE).unwrap() - kept.len());
    folder.write(FILE_NAME, &format!("{kept}{padding}"));

    assert_eq!(
        FilePromptHistory::new(Some(folder.path().to_path_buf())).load(),
        prompts(&["a"])
    );
}

#[test]
fn reads_a_file_of_four_mebibytes_but_not_more() {
    let folder = ScratchFolder::new("prompts-four");
    let kept = r#"{"format":1,"prompts":["a"]}"#;
    let padding = " ".repeat(4 * 1_024 * 1_024 - kept.len());
    let history = FilePromptHistory::new(Some(folder.path().to_path_buf()));

    folder.write(FILE_NAME, &format!("{kept}{padding}"));
    assert_eq!(history.load(), prompts(&["a"]));
    folder.write(FILE_NAME, &format!("{kept}{padding} "));
    assert_eq!(history.load(), Vec::<String>::new());
}
