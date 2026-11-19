use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::toolbox::tests::TempFolder;

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

#[test]
fn lists_two_levels_sorted_folders_marked() {
    let folder = TempFolder::new("folder-two-levels");
    folder.write("b.txt", "");
    folder.write("src/main.rs", "");
    folder.write("src/deep/inner.rs", "");
    folder.write("a.txt", "");

    let output = list(folder.path());
    let lines: Vec<&str> = text(&output).lines().skip(1).collect();

    assert_eq!(
        lines,
        ["a.txt", "b.txt", "src/", "src/deep/", "src/main.rs"]
    );
    assert_eq!(output.end, ToolEnd::Done(Detail::Files(5)));
    assert!(text(&output).starts_with(&format!("Entries of {}", folder.path().display())));
}

#[test]
fn leaves_out_hidden_entries_dependencies_and_build_output() {
    let folder = TempFolder::new("folder-skipped");
    folder.write(".git/HEAD", "");
    folder.write(".env", "");
    folder.write("node_modules/x/index.js", "");
    folder.write("target/debug/dot", "");
    folder.write("src/.hidden", "");
    folder.write("kept.rs", "");

    let output = list(folder.path());
    let lines: Vec<&str> = text(&output).lines().skip(1).collect();

    assert_eq!(lines, ["kept.rs", "src/"]);
}

#[test]
fn shows_a_thousand_entries_at_most() {
    let folder = TempFolder::new("folder-cap");
    for number in 0..1_001 {
        folder.write(&format!("f{number:04}.txt"), "");
    }
    let exact = TempFolder::new("folder-cap-exact");
    for number in 0..1_000 {
        exact.write(&format!("f{number:04}.txt"), "");
    }

    let capped = list(folder.path());
    let whole = list(exact.path());

    assert_eq!(capped.end, ToolEnd::Done(Detail::Files(1_000)));
    assert!(
        text(&capped)
            .ends_with("[Only the first 1000 entries are shown: list a subfolder to see more.]")
    );
    assert!(!text(&capped).contains("f1000.txt"));
    assert_eq!(whole.end, ToolEnd::Done(Detail::Files(1_000)));
    assert!(!text(&whole).contains("Only the first"));
}

#[test]
fn fails_on_a_folder_it_cannot_list() {
    let folder = TempFolder::new("folder-missing");

    let output = list(&folder.join("missing"));

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(text(&output).contains("Cannot list"));
}
