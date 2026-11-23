use std::os::unix::fs::symlink;

use dot_api::Usage;

use super::*;
use crate::git::tests::ScratchFolder;

const HEAD_PATH: &str = ".git/HEAD";

fn session_in(folder: &Path) -> LocalSession {
    LocalSession::new(Model::OPUS_5_5, Effort::High, Some(folder.to_path_buf()))
}

fn name_of(folder: &ScratchFolder) -> String {
    folder
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn reports_the_folder_and_its_branch() {
    let folder = ScratchFolder::new("project");
    folder.write(HEAD_PATH, "ref: refs/heads/feat/mac-app\n");

    let status = session_in(folder.path()).status();

    assert_eq!(status.project, Some(name_of(&folder)));
    assert_eq!(status.branch, Some("feat/mac-app".to_owned()));
}

#[test]
fn talks_to_the_model_it_was_given() {
    let status = LocalSession::new(Model::OPUS_5_5, Effort::High, None).status();

    assert_eq!(status.model, Model::OPUS_5_5);
}

#[test]
fn starts_with_nothing_consumed() {
    let folder = ScratchFolder::new("fresh");

    assert_eq!(session_in(folder.path()).status().usage, Usage::default());
}

#[test]
fn a_session_without_folder_has_no_project() {
    let status = LocalSession::new(Model::OPUS_5_5, Effort::High, None).status();

    assert_eq!((status.project, status.branch), (None, None));
}

#[test]
fn a_missing_folder_or_a_file_is_no_project() {
    let folder = ScratchFolder::new("not-a-folder");
    let file = folder.write("notes.txt", "");

    let missing = session_in(&folder.path().join("missing")).status();
    let not_a_folder = session_in(&file).status();

    assert_eq!((missing.project, missing.branch), (None, None));
    assert_eq!((not_a_folder.project, not_a_folder.branch), (None, None));
}

#[test]
fn names_a_linked_folder_after_the_real_one() {
    let real = ScratchFolder::new("real");
    let links = ScratchFolder::new("links");
    let alias = links.path().join("alias");
    symlink(real.path(), &alias).unwrap();

    let status = session_in(&alias).status();

    assert_eq!(status.project, Some(name_of(&real)));
}

#[test]
fn replaces_unreadable_characters_in_the_folder_name() {
    let parent = ScratchFolder::new("unreadable");
    let folder = parent.path().join("line\nbreak");
    fs::create_dir(&folder).unwrap();

    let status = session_in(&folder).status();

    assert_eq!(status.project, Some("line\u{fffd}break".to_owned()));
}

#[test]
fn replaces_invisible_reordering_characters_in_the_folder_name() {
    let parent = ScratchFolder::new("reordering");
    let folder = parent.path().join("dot\u{202e}tod");
    fs::create_dir(&folder).unwrap();

    let status = session_in(&folder).status();

    assert_eq!(status.project, Some("dot\u{fffd}tod".to_owned()));
}

#[test]
fn reads_the_branch_again_at_each_status() {
    let folder = ScratchFolder::new("checkout");
    folder.write(HEAD_PATH, "ref: refs/heads/main\n");
    let session = session_in(folder.path());
    let before = session.status();

    folder.write(HEAD_PATH, "ref: refs/heads/feat/status-bar\n");

    assert_eq!(before.branch, Some("main".to_owned()));
    assert_eq!(session.status().branch, Some("feat/status-bar".to_owned()));
}

#[test]
fn reports_the_effort_it_was_given() {
    let status = LocalSession::new(Model::OPUS_5_5, Effort::Max, None).status();

    assert_eq!(status.effort, Effort::Max);
}

fn reply(context_tokens: u64, micro_dollars: u64) -> Reply {
    Reply {
        context_tokens,
        cost: Cost::from_micro_dollars(micro_dollars),
        ..Reply::default()
    }
}

#[test]
fn adds_up_the_cost_of_replies_and_keeps_the_last_context() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);

    session.record(&reply(12_000, 1_250_000));
    session.record(&reply(48_000, 1_000_000));

    let usage = session.spending().usage;
    assert_eq!(usage.cost, Cost::from_micro_dollars(2_250_000));
    assert_eq!(usage.context_tokens, 48_000);
}

#[test]
fn a_reply_that_never_reached_claude_keeps_the_context() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);
    session.record(&reply(48_000, 1_000_000));

    session.record(&reply(0, 0));

    assert_eq!(session.spending().usage.context_tokens, 48_000);
}

#[test]
fn keeps_a_limit_a_later_reply_does_not_report() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);
    let limited = Reply {
        limits: dot_api::Limits {
            five_hours: Some(12),
            seven_days: Some(47),
        },
        ..Reply::default()
    };

    session.record(&limited);
    session.record(&Reply {
        limits: dot_api::Limits {
            five_hours: Some(13),
            seven_days: None,
        },
        ..Reply::default()
    });

    let limits = session.spending().limits;
    assert_eq!(limits.five_hours, Some(13));
    assert_eq!(limits.seven_days, Some(47));
}

#[test]
fn adds_up_the_lines_tools_changed() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);

    session.count_changes(3, 1);
    session.count_changes(10, 0);

    let usage = session.spending().usage;
    assert_eq!((usage.lines_added, usage.lines_removed), (13, 1));
}

#[test]
fn clearing_the_context_keeps_what_was_spent() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);
    session.record(&reply(48_000, 1_000_000));
    session.count_changes(2, 2);

    session.clear_context();

    let usage = session.spending().usage;
    assert_eq!(usage.context_tokens, 0);
    assert_eq!(usage.cost, Cost::from_micro_dollars(1_000_000));
    assert_eq!((usage.lines_added, usage.lines_removed), (2, 2));
}

#[test]
fn reports_what_was_spent_with_its_status() {
    let folder = ScratchFolder::new("spent");
    let session = session_in(folder.path());
    session.record(&Reply {
        limits: dot_api::Limits {
            five_hours: Some(5),
            seven_days: None,
        },
        ..reply(1_000, 40)
    });
    session.count_changes(1, 0);

    let status = session.status();

    assert_eq!(status.usage, session.spending().usage);
    assert_eq!(status.limits.five_hours, Some(5));
}

#[test]
fn sums_never_overflow() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);
    session.record(&reply(1, u64::MAX));
    session.record(&reply(1, 1));
    session.count_changes(u64::MAX, u64::MAX);
    session.count_changes(1, 1);

    let usage = session.spending().usage;
    assert_eq!(usage.cost, Cost::from_micro_dollars(u64::MAX));
    assert_eq!(
        (usage.lines_added, usage.lines_removed),
        (u64::MAX, u64::MAX)
    );
}

#[test]
fn keeps_the_base_of_the_conversation_until_it_starts_afresh() {
    let session = LocalSession::new(Model::OPUS_5_5, Effort::High, None);

    session.record(&Reply {
        base_tokens: Some(106_000),
        ..reply(120_000, 0)
    });
    session.record(&reply(150_000, 0));
    let carried_on = session.spending().usage;
    session.clear_context();
    let afresh = session.spending().usage;

    assert_eq!(carried_on.base_tokens, 106_000);
    assert_eq!(carried_on.context_tokens, 150_000);
    assert_eq!((afresh.base_tokens, afresh.context_tokens), (0, 0));
}
