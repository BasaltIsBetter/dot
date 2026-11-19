use std::fs::{self, OpenOptions};
use std::os::unix::fs::symlink;

use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::toolbox::tests::TempFolder;

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn asking(pattern: &str, show: SearchShow) -> TextSearch {
    TextSearch {
        pattern: pattern.to_owned(),
        folder: None,
        files: None,
        ignore_case: false,
        show,
    }
}

fn searched(folder: &Path, search_for: &TextSearch) -> ToolOutput {
    search(folder, search_for, &Cancellation::new())
}

fn project() -> TempFolder {
    let folder = TempFolder::new("text-project");
    folder.write("src/a.rs", "fn main() {\n    Needle();\n}\n");
    folder.write("src/b.rs", "needle\nneedle\r\nhay\n");
    folder.write("notes.txt", "needle in notes\n");
    folder
}

#[test]
fn lists_the_files_that_match() {
    let folder = project();

    let output = searched(folder.path(), &asking("needle", SearchShow::Files));

    assert_eq!(text(&output), "notes.txt\nsrc/b.rs\n");
    assert_eq!(output.end, ToolEnd::Done(Detail::Files(2)));
}

#[test]
fn shows_each_matching_line_with_its_file_and_number() {
    let folder = project();

    let output = searched(folder.path(), &asking("needle", SearchShow::Lines));

    assert_eq!(
        text(&output),
        "notes.txt:1:needle in notes\nsrc/b.rs:1:needle\nsrc/b.rs:2:needle\n"
    );
    assert_eq!(output.end, ToolEnd::Done(Detail::Matches(3)));
}

#[test]
fn counts_the_matching_lines_of_each_file() {
    let folder = project();

    let output = searched(folder.path(), &asking("needle", SearchShow::Counts));

    assert_eq!(text(&output), "notes.txt:1\nsrc/b.rs:2\n");
    assert_eq!(output.end, ToolEnd::Done(Detail::Matches(3)));
}

#[test]
fn ignores_case_when_asked() {
    let folder = project();
    let search_for = TextSearch {
        ignore_case: true,
        ..asking("needle\\(", SearchShow::Lines)
    };

    let output = searched(folder.path(), &search_for);

    assert_eq!(text(&output), "src/a.rs:2:    Needle();\n");
}

#[test]
fn keeps_to_the_files_a_glob_names() {
    let folder = project();
    let by_name = TextSearch {
        files: Some("*.rs".to_owned()),
        ..asking("needle", SearchShow::Files)
    };
    let by_path = TextSearch {
        files: Some("notes.*".to_owned()),
        ..asking("needle", SearchShow::Files)
    };
    let broken = TextSearch {
        files: Some("[".to_owned()),
        ..asking("needle", SearchShow::Files)
    };

    assert_eq!(text(&searched(folder.path(), &by_name)), "src/b.rs\n");
    assert_eq!(text(&searched(folder.path(), &by_path)), "notes.txt\n");
    assert_eq!(searched(folder.path(), &broken).end, ToolEnd::Failed);
}

#[test]
fn searches_a_single_file_when_given_one() {
    let folder = project();

    let output = searched(
        &folder.join("src/b.rs"),
        &asking("needle", SearchShow::Counts),
    );

    assert_eq!(text(&output), "b.rs:2\n");
}

#[test]
fn says_when_nothing_matches_and_refuses_a_broken_expression() {
    let folder = project();

    let nothing = searched(folder.path(), &asking("absent", SearchShow::Lines));
    let broken = searched(folder.path(), &asking("(", SearchShow::Files));

    assert!(text(&nothing).starts_with("Nothing under"));
    assert_eq!(nothing.end, ToolEnd::Done(Detail::Matches(0)));
    assert_eq!(
        searched(folder.path(), &asking("absent", SearchShow::Files)).end,
        ToolEnd::Done(Detail::Files(0))
    );
    assert_eq!(broken.end, ToolEnd::Failed);
    assert!(text(&broken).contains("not valid"));
}

#[test]
fn skips_binary_files_large_files_links_and_ignored_files() {
    let folder = TempFolder::new("text-skipped");
    fs::write(folder.join("data.bin"), b"needle\0").unwrap();
    let large = folder.write("large.txt", &format!("needle\n{}", "a".repeat(9_000)));
    OpenOptions::new()
        .write(true)
        .open(&large)
        .unwrap()
        .set_len(LARGEST_SEARCHED + 1)
        .unwrap();
    let largest = folder.write("largest.txt", &format!("needle\n{}", "a".repeat(9_000)));
    OpenOptions::new()
        .write(true)
        .open(&largest)
        .unwrap()
        .set_len(LARGEST_SEARCHED)
        .unwrap();
    let elsewhere = TempFolder::new("text-skipped-other");
    let outside = elsewhere.write("outside.txt", "needle\n");
    symlink(&outside, folder.join("door.txt")).unwrap();
    folder.write(".gitignore", "ignored.txt\n");
    folder.write("ignored.txt", "needle\n");

    let output = searched(folder.path(), &asking("needle", SearchShow::Files));

    assert_eq!(text(&output), "largest.txt\n");
}

#[test]
fn cuts_long_lines_and_caps_the_lines_shown() {
    let folder = TempFolder::new("text-caps");
    let long = format!("needle{}", "é".repeat(600));
    folder.write("long.txt", &format!("{long}\n"));
    folder.write("many.txt", &"needle\n".repeat(600));

    let output = searched(folder.path(), &asking("needle", SearchShow::Lines));
    let lines: Vec<&str> = text(&output).lines().collect();

    assert!(lines[0].starts_with("long.txt:1:needle"));
    assert!(lines[0].ends_with(" […]"));
    assert_eq!(
        lines[0].chars().count(),
        "long.txt:1:".len() + 500 + " […]".chars().count()
    );
    assert_eq!(lines.len(), 501);
    assert_eq!(
        lines[500],
        "[Only the first 500 lines are shown: narrow the search to see the rest.]"
    );
    assert_eq!(output.end, ToolEnd::Done(Detail::Matches(601)));
}

#[test]
fn caps_the_files_listed() {
    let folder = TempFolder::new("text-file-cap");
    for number in 0..201 {
        folder.write(&format!("f{number:03}.txt"), "needle\n");
    }

    let files = searched(folder.path(), &asking("needle", SearchShow::Files));
    let counts = searched(folder.path(), &asking("needle", SearchShow::Counts));

    assert_eq!(files.end, ToolEnd::Done(Detail::Files(201)));
    assert!(
        text(&files)
            .ends_with("[Only the first 200 files are shown: narrow the search to see the rest.]")
    );
    assert!(!text(&files).contains("f200.txt"));
    assert!(text(&counts).contains("Only the first 200 files"));
}

#[test]
fn stops_when_the_reply_is_stopped() {
    let folder = project();
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let output = search(
        folder.path(),
        &asking("needle", SearchShow::Files),
        &cancellation,
    );

    assert_eq!(output, ToolOutput::stopped());
}

#[test]
fn skips_a_named_pipe_without_waiting_on_it() {
    let folder = project();
    folder.pipe("src/fifo.rs");
    let root = folder.path().to_path_buf();

    let output = crate::toolbox::tests::without_waiting(move || {
        searched(&root, &asking("needle", SearchShow::Files))
    });

    assert_eq!(text(&output), "notes.txt\nsrc/b.rs\n");
}
