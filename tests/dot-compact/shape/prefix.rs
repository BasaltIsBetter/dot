use proptest::prelude::*;

use super::*;

fn expanded(named: &str) -> String {
    let mut folder: Option<&str> = None;
    let mut lines = Vec::new();
    for line in named.split('\n') {
        let noted = line
            .strip_prefix("[paths under ")
            .and_then(|rest| rest.strip_suffix(']'));
        match (noted, folder, line.strip_prefix(' ')) {
            (Some(noted), _, _) => folder = Some(noted),
            (None, Some(folder), Some(rest)) => lines.push(format!("{folder}{rest}")),
            _ => {
                folder = None;
                lines.push(line.to_owned());
            }
        }
    }
    lines.join("\n")
}

#[test]
fn finds_the_folder_absolute_paths_share() {
    let paths = [
        "/Users/me/project/src/a.rs",
        "/Users/me/project/src/sub/b.rs",
        "/Users/me/project/c.rs",
    ];

    assert_eq!(common_folder(paths), Some("/Users/me/project/"));
}

#[test]
fn stops_at_a_whole_folder_name() {
    assert_eq!(
        common_folder(["/Users/me/project-one/a.rs", "/Users/me/project-two/b.rs"]),
        None
    );
    assert_eq!(
        common_folder([
            "/Users/me/a-long-folder/project-one/a.rs",
            "/Users/me/a-long-folder/project-two/b.rs"
        ]),
        Some("/Users/me/a-long-folder/")
    );
}

#[test]
fn names_no_folder_when_there_is_too_little_to_gain() {
    assert_eq!(common_folder(["src/a.rs", "src/b.rs"]), None);
    assert_eq!(common_folder(["/Users/me/project/src/a.rs"]), None);
    assert_eq!(common_folder(["/tmp/a.rs", "/tmp/b.rs"]), None);
    assert_eq!(
        common_folder(["/Users/me/project/a.rs", "relative/b.rs"]),
        None
    );
    assert_eq!(common_folder([""; 0]), None);
}

#[test]
fn the_note_names_the_folder() {
    assert_eq!(
        note("/Users/me/project/"),
        "[paths under /Users/me/project/]"
    );
}

#[test]
fn finds_the_folder_relative_paths_share() {
    assert_eq!(
        shared_folder([
            "crates/dot-compact/src/lib.rs",
            "crates/dot-compact/src/note.rs"
        ]),
        Some("crates/dot-compact/src/")
    );
    assert_eq!(shared_folder(["crates/a/src/lib.rs", "docs/b.md"]), None);
    assert_eq!(shared_folder(["src/a.rs", "src/b.rs"]), None);
    assert_eq!(shared_folder(["a-long-enough-folder/src/a.rs"]), None);
    assert_eq!(shared_folder([""; 0]), None);
}

#[test]
fn names_once_the_folder_neighbouring_lines_start_with() {
    let found = "core/src/main/java/app/chat/Channel.java\ncore/src/main/java/app/chat/listener/Join.java\ncore/src/main/java/app/role/Role.java:12:    private final String name;\nREADME.md\n";

    assert_eq!(
        name_neighbours(found).as_deref(),
        Some(
            "[paths under core/src/main/java/app/]\n chat/Channel.java\n chat/listener/Join.java\n role/Role.java:12:    private final String name;\nREADME.md\n"
        )
    );
}

#[test]
fn names_each_run_of_neighbours_and_keeps_the_end_of_the_text() {
    let found = "a-long-enough-folder/one/a\na-long-enough-folder/one/b\na-long-enough-folder/two/c\nplain text\nanother-long-folder/x/a\nanother-long-folder/x/b\nanother-long-folder/x/c";

    assert_eq!(
        name_neighbours(found).as_deref(),
        Some(
            "[paths under a-long-enough-folder/]\n one/a\n one/b\n two/c\nplain text\n[paths under another-long-folder/x/]\n a\n b\n c"
        )
    );
}

#[test]
fn does_not_reach_for_a_folder_further_up_when_it_would_gain_less() {
    let found = "a-long-enough-folder/with/a/deep/sub/folder/a\na-long-enough-folder/with/a/deep/sub/folder/b\na-long-enough-folder/with/a/deep/sub/folder/c\na-long-enough-folder/d\n";

    assert_eq!(
        name_neighbours(found).as_deref(),
        Some(
            "[paths under a-long-enough-folder/with/a/deep/sub/folder/]\n a\n b\n c\na-long-enough-folder/d\n"
        )
    );
}

#[test]
fn reaches_for_a_folder_further_up_when_it_gains_as_much() {
    let found = "a-long-enough-folder/x/a\na-long-enough-folder/x/b\na-long-enough-folder/c\n";

    assert_eq!(
        name_neighbours(found).as_deref(),
        Some("[paths under a-long-enough-folder/]\n x/a\n x/b\n c\n")
    );
}

#[test]
fn leaves_alone_two_neighbours_short_folders_and_what_an_indented_line_follows() {
    let two = "a-long-enough-folder/a\na-long-enough-folder/b\n";
    let three = format!("{two}a-long-enough-folder/c\n");
    let exactly_sixteen = "fifteen-letters/a\nfifteen-letters/b\nfifteen-letters/c\n";
    let fifteen = "fourteen-chars/a\nfourteen-chars/b\nfourteen-chars/c\n";

    assert_eq!(name_neighbours(two), None);
    assert!(name_neighbours(&three).is_some());
    assert_eq!(name_neighbours(&format!("{three}  indented\n")), None);
    assert_eq!(name_neighbours(&format!("{three}\tindented\n")), None);
    assert_eq!(
        name_neighbours(&format!("{three}{three}  indented\n{three}")).as_deref(),
        Some(
            format!("{three}{three}  indented\n[paths under a-long-enough-folder/]\n a\n b\n c\n")
                .as_str()
        )
    );
    assert!(name_neighbours(&format!("{three}\nafter a blank line\n")).is_some());
    assert!(name_neighbours(exactly_sixteen).is_some());
    assert_eq!(name_neighbours(fifteen), None);
    assert_eq!(
        name_neighbours(
            "https://a-long-enough-host/a\nhttps://a-long-enough-host/b\nhttps://a-long-enough-host/c\n"
        ),
        None
    );
    assert_eq!(
        name_neighbours(
            " a-long-enough-folder/a\n a-long-enough-folder/b\n a-long-enough-folder/c\n"
        ),
        None
    );
    assert_eq!(name_neighbours(""), None);
}

#[test]
fn only_the_path_a_line_starts_with_names_its_folder() {
    let found = "a-long-enough-folder/x.rs: see docs/one\na-long-enough-folder/x.rs: see docs/two\na-long-enough-folder/x.rs\tsee docs/six\na-long-enough-folder/x.rs see docs/ten\n";

    assert_eq!(
        name_neighbours(found).as_deref(),
        Some(
            "[paths under a-long-enough-folder/]\n x.rs: see docs/one\n x.rs: see docs/two\n x.rs\tsee docs/six\n x.rs see docs/ten\n"
        )
    );
}

#[test]
fn shares_a_folder_by_whole_characters_and_whole_names() {
    assert_eq!(
        folder_in_common("dossier/été/a", "dossier/étè/b"),
        "dossier/"
    );
    assert_eq!(
        folder_in_common("folder/sub-one/a", "folder/sub-two/b"),
        "folder/"
    );
    assert_eq!(
        folder_in_common("folder/sub/", "folder/sub/deeper/"),
        "folder/sub/"
    );
    assert_eq!(folder_in_common("one/a", "two/b"), "");
}

proptest! {
    #[test]
    fn every_line_is_found_again_under_its_note(
        lines in prop::collection::vec(
            "((a-long-enough-folder|another-long-folder)/((sub|other)/){0,2})?[a-z]{1,6}(:[0-9]{1,2}:[a-z ]{0,8})?",
            0..30,
        ),
        ends_with_a_line_break in any::<bool>(),
    ) {
        let mut text = lines.join("\n");
        if ends_with_a_line_break && !text.is_empty() {
            text.push('\n');
        }

        let named = name_neighbours(&text).unwrap_or_else(|| text.clone());

        prop_assert!(named.len() <= text.len());
        prop_assert_eq!(expanded(&named), text);
    }
}
