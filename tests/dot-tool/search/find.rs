use std::fs;
use std::os::unix::fs::symlink;
use std::time::Duration;

use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::toolbox::tests::TempFolder;

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn listed(output: &ToolOutput) -> Vec<&str> {
    text(output).lines().skip(1).collect()
}

#[test]
fn finds_files_newest_first() {
    let folder = TempFolder::new("find-newest");
    folder.write("src/old.rs", "");
    std::thread::sleep(Duration::from_millis(20));
    folder.write("src/new.rs", "");
    folder.write("README.md", "");

    let output = find(folder.path(), "src/*.rs", &Cancellation::new());

    assert_eq!(listed(&output), ["src/new.rs", "src/old.rs"]);
    assert_eq!(output.end, ToolEnd::Done(Detail::Files(2)));
    assert!(text(&output).starts_with(&format!(
        "Files under {} that match src/*.rs, newest first:",
        folder.path().display()
    )));
}

#[test]
fn respects_gitignore_and_leaves_hidden_files_out() {
    let folder = TempFolder::new("find-ignored");
    folder.write(".gitignore", "build/\n*.log\n");
    folder.write("build/out.rs", "");
    folder.write("debug.log", "");
    folder.write(".secret.rs", "");
    folder.write("kept.rs", "");

    let rust = find(folder.path(), "**/*.rs", &Cancellation::new());
    let logs = find(folder.path(), "*.log", &Cancellation::new());

    assert_eq!(listed(&rust), ["kept.rs"]);
    assert_eq!(logs.end, ToolEnd::Done(Detail::Files(0)));
}

#[test]
fn says_when_nothing_matches() {
    let folder = TempFolder::new("find-nothing");

    let output = find(folder.path(), "*.rs", &Cancellation::new());

    assert_eq!(
        text(&output),
        format!("No file under {} matches *.rs.", folder.path().display())
    );
}

#[test]
fn lists_two_hundred_files_at_most() {
    let folder = TempFolder::new("find-cap");
    for number in 0..201 {
        folder.write(&format!("f{number:03}.txt"), "");
    }

    let output = find(folder.path(), "*.txt", &Cancellation::new());

    assert_eq!(listed(&output).len(), 201);
    assert_eq!(output.end, ToolEnd::Done(Detail::Files(201)));
    assert!(text(&output).ends_with("[1 more files match: narrow the pattern to see them.]"));
}

#[test]
fn takes_a_pattern_written_from_the_root_of_the_disk() {
    let folder = TempFolder::new("find-absolute");
    folder.write("src/a.rs", "");
    let pattern = format!("{}/src/*.rs", folder.path().display());

    let output = find(folder.path(), &pattern, &Cancellation::new());

    assert_eq!(listed(&output), ["src/a.rs"]);
}

#[test]
fn refuses_a_broken_pattern_or_a_root_that_is_no_folder() {
    let folder = TempFolder::new("find-refused");
    let file = folder.write("a.txt", "");

    let broken = find(folder.path(), "[", &Cancellation::new());
    let not_a_folder = find(&file, "*", &Cancellation::new());

    assert_eq!(broken.end, ToolEnd::Failed);
    assert_eq!(not_a_folder.end, ToolEnd::Failed);
    assert!(text(&not_a_folder).contains("is not a folder"));
}

#[test]
fn stops_when_the_reply_is_stopped() {
    let folder = TempFolder::new("find-stopped");
    folder.write("a.txt", "");
    let cancellation = Cancellation::new();
    cancellation.cancel();

    assert_eq!(
        find(folder.path(), "*", &cancellation),
        ToolOutput::stopped()
    );
}

#[test]
fn does_not_follow_links_into_other_folders() {
    let folder = TempFolder::new("find-links");
    let elsewhere = TempFolder::new("find-links-other");
    elsewhere.write("outside.rs", "");
    symlink(elsewhere.path(), folder.join("door")).unwrap();
    fs::write(folder.join("inside.rs"), "").unwrap();

    let output = find(folder.path(), "**/*.rs", &Cancellation::new());

    assert_eq!(listed(&output), ["inside.rs"]);
}
