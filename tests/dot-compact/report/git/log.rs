use super::*;

const LOG: &str = "commit dbe543bc0d2fc10a06a3b4c2c6cb6e0945c0c232\nAuthor: Jules <jules@example.com>\nDate:   Thu Nov 19 18:36:05 2026 +0100\n\n    test(desktop): pin the screen switch\n\ncommit 305a7ed0d2fc10a06a3b4c2c6cb6e0945c0c2321\nMerge: 1184bc8 50018c0\nAuthor: Jules <jules@example.com>\nDate:   Thu Nov 19 17:02:11 2026 +0100\n\n    refactor: take toolbox by mut\n\n    A second paragraph of the message.\n";

fn log(command: &str, text: &str) -> Option<String> {
    compact(&CommandLine::read(command), text)
}

#[test]
fn writes_once_the_author_of_every_commit() {
    assert_eq!(
        log("git log -2", LOG).as_deref(),
        Some(
            "[git log: every commit by Jules <jules@example.com>]\ncommit dbe543bc0d2fc10a06a3b4c2c6cb6e0945c0c232\nDate:   Thu Nov 19 18:36:05 2026 +0100\n    test(desktop): pin the screen switch\ncommit 305a7ed0d2fc10a06a3b4c2c6cb6e0945c0c2321\nMerge: 1184bc8 50018c0\nDate:   Thu Nov 19 17:02:11 2026 +0100\n    refactor: take toolbox by mut\n\n    A second paragraph of the message.\n"
        )
    );
}

#[test]
fn keeps_the_log_of_several_authors_as_it_is() {
    let two_authors = LOG.replacen("Jules <jules@example.com>", "Sam <sam@example.com>", 1);

    assert_eq!(log("git log -2", &two_authors), None);
}

#[test]
fn keeps_a_single_commit_as_it_is() {
    let one_commit = LOG.split("\ncommit 305a7ed").next().unwrap();

    assert_eq!(log("git log -1", one_commit), None);
}

#[test]
fn only_reads_what_git_log_prints_in_its_own_format() {
    let one_line =
        "dbe543b test(desktop): pin the screen switch\n305a7ed refactor: take toolbox by mut\n";

    assert_eq!(log("git log --oneline", one_line), None);
    assert_eq!(log("git shortlog", LOG), None);
    assert_eq!(log("cargo test", LOG), None);
    assert_eq!(log("git log", ""), None);
    assert!(log("cd repo && git log", LOG).is_some());
}

#[test]
fn a_commit_starts_with_its_hash() {
    assert!(starts_a_commit(
        "commit dbe543bc0d2fc10a06a3b4c2c6cb6e0945c0c232"
    ));
    assert!(starts_a_commit("commit dbe543b (HEAD -> main)"));
    for line in [
        "commit dbe543",
        "commit zzzzzzz",
        "commit ",
        "committed dbe543bc",
        "    commit dbe543bc",
    ] {
        assert!(!starts_a_commit(line), "{line}");
    }
}

#[test]
fn only_the_empty_lines_around_a_message_frame_it() {
    let lines = [
        "Date:   Thu Nov 19 18:36:05 2026 +0100",
        "",
        "    first paragraph",
        "",
        "    second paragraph",
        "",
        "commit 305a7ed0d2fc10a06a3b4c2c6cb6e0945c0c2321",
        "",
    ];

    let framing: Vec<bool> = (0..lines.len())
        .map(|at| frames_a_message(&lines, at))
        .collect();

    assert_eq!(
        framing,
        [false, true, false, false, false, true, false, false]
    );
}
