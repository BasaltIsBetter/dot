use super::*;

const DIFF: &str = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,12 +1,12 @@\n one\n two\n three\n four\n five\n-old\n+new\n six\n seven\n eight\n nine\n ten\n";
const LOG: &str = "commit aaa\nAuthor: A <a@x>\nDate:   Mon\n\n    subject one\n    \n    body line 1\n    body line 2\n\ncommit bbb\nAuthor: A <a@x>\nDate:   Tue\n\n    subject two\n";

#[test]
fn keeps_the_line_before_and_the_line_after_each_change_of_a_diff() {
    assert_eq!(
        far_context(DIFF).as_deref(),
        Some(
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,12 +1,12 @@\n[+4 unchanged lines]\n five\n-old\n+new\n six\n[+4 unchanged lines]\n"
        )
    );
}

#[test]
fn leaves_alone_two_far_lines_and_what_is_not_a_diff() {
    let short = "diff --git a/a.rs b/a.rs\n@@ -1,6 +1,6 @@\n one\n two\n three\n-old\n+new\n four\n five\n six\n";
    let not_a_diff = " one\n two\n three\n four\n five\n-old\n+new\n six\n";

    assert_eq!(far_context(short), None);
    assert_eq!(far_context(not_a_diff), None);
    assert_eq!(far_context(""), None);
}

#[test]
fn the_names_of_the_two_files_are_not_changes() {
    assert!(is_changed("+new"));
    assert!(is_changed("-old"));
    assert!(is_changed("+"));
    assert!(!is_changed("+++ b/a.rs"));
    assert!(!is_changed("--- a/a.rs"));
    assert!(!is_changed(" context"));
    assert!(!is_changed("@@ -1 +1 @@"));
}

#[test]
fn keeps_the_subject_of_each_commit_and_counts_the_rest_of_its_message() {
    assert_eq!(
        messages(LOG).as_deref(),
        Some(
            "commit aaa\nAuthor: A <a@x>\nDate:   Mon\n\n    subject one\n[+4 lines of message]\ncommit bbb\nAuthor: A <a@x>\nDate:   Tue\n\n    subject two\n"
        )
    );
}

#[test]
fn leaves_alone_one_commit_short_messages_and_what_is_not_a_log() {
    let one = "commit aaa\n\n    subject\n    \n    body 1\n    body 2\n    body 3\n";
    let short = "commit aaa\n\n    subject\n    body\n\ncommit bbb\n\n    subject\n";
    let indented = "    one\n    two\n    three\n    four\n    five\n";

    assert_eq!(messages(one), None);
    assert_eq!(messages(short), None);
    assert_eq!(messages(indented), None);
    assert_eq!(messages(""), None);
}

#[test]
fn reads_once_the_empty_lines_under_a_message_however_many() {
    let log = format!(
        "commit 1\n\n    subject\n{}commit 2\n\n    subject\n",
        "\n".repeat(300_000)
    );

    let thinner = messages(&log).unwrap();

    assert_eq!(
        thinner,
        "commit 1\n\n    subject\n[+300000 lines of message]\ncommit 2\n\n    subject\n"
    );
}

const FIRST: &str = "abcdef0123456789abcdef0123456789abcdef01";
const SECOND: &str = "1234567890abcdef1234567890abcdef12345678";

#[test]
fn writes_one_line_per_commit_of_a_log_by_one_author() {
    let log = format!(
        "[git log: every commit by Ann <ann@example.com>]\ncommit {FIRST} (HEAD -> main)\nDate:   Thu Nov 19 18:36:05 2026 +0100\n    add the relay\n    It ends with the command.\n    Not with what it left running.\ncommit {SECOND}\nDate:   Wed Nov 18 09:00:00 2026 +0100\n    fix the pause\n"
    );

    assert_eq!(
        commits(&log).as_deref(),
        Some(
            "[git log: every commit by Ann <ann@example.com>]\n[git log as: hash day subject, one line per commit]\nabcdef0 (HEAD -> main) Nov 19 2026 add the relay [+2 lines]\n1234567 Nov 18 2026 fix the pause\n"
        )
    );
}

#[test]
fn names_the_author_of_each_commit_when_git_wrote_it() {
    let log = format!(
        "commit {FIRST}\nMerge: abcdef0 1234567\nAuthor: Ann Lee <ann@example.com>\nDate:   Thu Nov 19 18:36:05 2026 +0100\n\n    merge the relay\n\ncommit {SECOND}\nAuthor: Bob <bob@example.com>\nDate:   2026-11-18\n\n    fix the pause\n\n    Longer words.\n"
    );

    assert_eq!(
        commits(log.trim_end()).as_deref(),
        Some(
            "[git log as: hash day subject, one line per commit]\nabcdef0 Nov 19 2026 <Ann Lee> merge the relay\n1234567 2026-11-18 <Bob> fix the pause [+1 lines]"
        )
    );
}

#[test]
fn leaves_alone_a_log_it_cannot_write_one_line_per_commit() {
    let one = format!("commit {FIRST}\nDate:   Thu Nov 19 18:36:05 2026 +0100\n    only one\n");
    let two = format!("{one}commit {SECOND}\nDate:   Wed Nov 18 09:00:00 2026 +0100\n    second\n");
    let with_changed_files = format!("{two} src/lib.rs | 2 +-\n");
    let without_a_date = format!("{one}commit {SECOND}\n    second\n");
    let without_a_subject =
        format!("{one}commit {SECOND}\nDate:   Wed Nov 18 09:00:00 2026 +0100\n");
    let not_hashes =
        "commit the first change\nDate:   x\n    a\ncommit the second change\nDate:   y\n    b\n";
    let longer_than_a_hash = two.replace(SECOND, &format!("{SECOND}9"));

    assert!(commits(&two).is_some());
    assert_eq!(commits(&one), None);
    assert_eq!(commits(&with_changed_files), None);
    assert_eq!(commits(&without_a_date), None);
    assert_eq!(commits(&without_a_subject), None);
    assert_eq!(commits(not_hashes), None);
    assert_eq!(commits(&longer_than_a_hash), None);
    assert_eq!(commits("no log here\n"), None);
}

#[test]
fn keeps_what_another_command_printed_before_the_log() {
    let log = format!(
        "On branch main\ncommit {FIRST}\nDate:   Thu Nov 19 18:36:05 2026 +0100\n    first\ncommit {SECOND}\nDate:   Wed Nov 18 09:00:00 2026 +0100\n    second\n"
    );

    assert!(commits(&log).unwrap().starts_with(
        "On branch main\n[git log as: hash day subject, one line per commit]\nabcdef0 Nov 19 2026 first\n"
    ));
}
