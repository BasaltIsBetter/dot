use std::fs;

use dot_api::{Detail, ToolEnd};

use super::*;
use crate::tool::tests::Untouched;

fn started(id: &str, command: &str) -> ReplyEvent {
    ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool: ToolUse {
            kind: ToolKind::Run,
            subject: command.to_owned(),
        },
    }
}

fn ended(id: &str) -> ReplyEvent {
    ReplyEvent::ToolEnded {
        id: id.to_owned(),
        end: ToolEnd::Done(Detail::Exit(0)),
    }
}

fn output(id: &str, lines: &[&str]) -> ReplyEvent {
    ReplyEvent::ToolOutput {
        id: id.to_owned(),
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
    }
}

#[test]
fn shows_what_a_command_prints_under_the_call_that_runs_it() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    live.notice(&started("toolu_2", "cargo test"));

    let first = live.heard(0, "cargo test", b"Compiling\n");
    let more = live.heard(0, "cargo test", b"Finished\n");

    assert_eq!(first, Some(output("toolu_2", &["Compiling"])));
    assert_eq!(more, Some(output("toolu_2", &["Compiling", "Finished"])));
}

#[test]
fn takes_the_only_running_command_when_the_names_differ() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "cargo test"));

    let heard = live.heard(0, "cargo  test", b"ok\n");

    assert_eq!(heard, Some(output("toolu_1", &["ok"])));
}

#[test]
fn shows_nothing_when_it_cannot_tell_which_command_prints() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    live.notice(&started("toolu_2", "pwd"));

    assert_eq!(live.heard(0, "whoami", b"me\n"), None);
}

#[test]
fn never_shows_a_command_of_an_agent_as_one_of_claude() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "cargo test"));
    live.notice(&ReplyEvent::Agent {
        id: "toolu_7".to_owned(),
        event: AgentEvent::ToolStarted {
            id: "toolu_8".to_owned(),
            tool: ToolUse {
                kind: ToolKind::Run,
                subject: "ls".to_owned(),
            },
        },
    });

    let of_the_agent = live.heard(0, "ls", b"a.txt\n");
    let of_claude = live.heard(1, "cargo test", b"ok\n");

    assert_eq!(of_the_agent, None);
    assert_eq!(of_claude, Some(output("toolu_1", &["ok"])));
}

#[test]
fn shows_nothing_once_the_command_ended_or_its_connection_closed() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    live.heard(0, "ls", b"a\n");

    let closed = live.heard(0, "ls", b"");
    live.heard(1, "ls", b"b\n");
    live.notice(&ended("toolu_1"));
    let after_the_end = live.heard(1, "ls", b"c\n");

    assert_eq!(closed, None);
    assert_eq!(after_the_end, None);
}

#[test]
fn keeps_following_a_command_when_another_one_closes() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "make"));
    live.notice(&started("toolu_2", "cargo test"));
    live.heard(0, "make", b"cc a.c\n");
    live.heard(1, "cargo test", b"Compiling\n");

    live.heard(0, "make", b"");
    let more = live.heard(1, "cargo test", b"Finished\n");

    assert_eq!(more, Some(output("toolu_2", &["Compiling", "Finished"])));
}

#[test]
fn ignores_the_tools_that_run_no_command() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&ReplyEvent::ToolStarted {
        id: "toolu_1".to_owned(),
        tool: ToolUse {
            kind: ToolKind::Read,
            subject: "ls".to_owned(),
        },
    });

    assert_eq!(live.heard(0, "ls", b"x\n"), None);
}

#[test]
fn listens_in_a_folder_of_its_own_inside_dot_s_workspace() {
    let workspace = std::env::temp_dir().join(format!("dot-live-workspace-{}", std::process::id()));

    let outputs = live_outputs(Some(&workspace)).unwrap();

    assert!(outputs.address().starts_with(workspace.join("live")));
    assert!(live_outputs(None).is_none());
    drop(outputs);
    fs::remove_dir_all(&workspace).unwrap();
}

#[test]
fn names_its_socket_after_the_process_and_eight_hexadecimal_digits() {
    let workspace = std::env::temp_dir().join(format!("dot-live-name-{}", std::process::id()));

    let outputs = live_outputs(Some(&workspace)).unwrap();
    let name = outputs
        .address()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let (process, random) = name.split_once('-').unwrap();
    assert_eq!(process, std::process::id().to_string());
    assert_eq!(random.len(), 8);
    assert!(random.chars().all(|digit| digit.is_ascii_hexdigit()));
    drop(outputs);
    fs::remove_dir_all(&workspace).unwrap();
}
