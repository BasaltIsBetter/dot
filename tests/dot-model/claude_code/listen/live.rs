use std::cell::Cell;
use std::fs;
use std::sync::OnceLock;

use dot_api::{Detail, ToolEnd};

use super::*;
use crate::tool::tests::Untouched;

fn later() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    thread_local! {
        static STEP: Cell<u32> = const { Cell::new(0) };
    }
    let step = STEP.with(|step| {
        step.set(step.get() + 1);
        step.get()
    });
    *START.get_or_init(Instant::now) + SHOW_EVERY * 2 * step
}

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

    let first = live.heard(0, "cargo test", b"Compiling\n", later());
    let more = live.heard(0, "cargo test", b"Finished\n", later());

    assert_eq!(first, Some(output("toolu_2", &["Compiling"])));
    assert_eq!(more, Some(output("toolu_2", &["Compiling", "Finished"])));
}

#[test]
fn takes_the_only_running_command_when_the_names_differ() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "cargo test"));

    let heard = live.heard(0, "cargo  test", b"ok\n", later());

    assert_eq!(heard, Some(output("toolu_1", &["ok"])));
}

#[test]
fn shows_nothing_when_it_cannot_tell_which_command_prints() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    live.notice(&started("toolu_2", "pwd"));

    assert_eq!(live.heard(0, "whoami", b"me\n", later()), None);
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

    let of_the_agent = live.heard(0, "ls", b"a.txt\n", later());
    let of_claude = live.heard(1, "cargo test", b"ok\n", later());

    assert_eq!(of_the_agent, None);
    assert_eq!(of_claude, Some(output("toolu_1", &["ok"])));
}

#[test]
fn shows_nothing_once_the_command_ended_or_its_connection_closed() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    live.heard(0, "ls", b"a\n", later());

    let closed = live.heard(0, "ls", b"", later());
    live.heard(1, "ls", b"b\n", later());
    live.notice(&ended("toolu_1"));
    let after_the_end = live.heard(1, "ls", b"c\n", later());

    assert_eq!(closed, None);
    assert_eq!(after_the_end, None);
}

#[test]
fn keeps_following_a_command_when_another_one_closes() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "make"));
    live.notice(&started("toolu_2", "cargo test"));
    live.heard(0, "make", b"cc a.c\n", later());
    live.heard(1, "cargo test", b"Compiling\n", later());

    live.heard(0, "make", b"", later());
    let more = live.heard(1, "cargo test", b"Finished\n", later());

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

    assert_eq!(live.heard(0, "ls", b"x\n", later()), None);
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

#[test]
fn shows_what_a_command_prints_a_tenth_of_a_second_apart_at_most() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "yes"));
    let start = Instant::now();
    let soon = start + SHOW_EVERY / 2;

    let first = live.heard(0, "yes", b"y\n", start);
    let too_soon = live.heard(0, "yes", b"z\n", soon);
    let still_waiting = live.due(soon);
    let shown = live.due(start + SHOW_EVERY);
    let nothing_more = live.due(start + SHOW_EVERY * 3);

    assert_eq!(first, Some(output("toolu_1", &["y"])));
    assert_eq!(too_soon, None);
    assert_eq!(still_waiting, []);
    assert_eq!(shown, [output("toolu_1", &["y", "z"])]);
    assert_eq!(nothing_more, []);
}

#[test]
fn shows_what_waited_when_the_command_ends() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    let start = Instant::now();

    live.heard(0, "ls", b"a\n", start);
    live.heard(0, "ls", b"b\n", start);
    let at_the_end = live.heard(0, "ls", b"", start);

    assert_eq!(at_the_end, Some(output("toolu_1", &["a", "b"])));
    assert_eq!(live.due(start + SHOW_EVERY), []);
}

#[test]
fn shows_nothing_more_at_the_end_of_a_command_already_shown() {
    let mut live = LiveCommands::new(&Untouched);
    live.notice(&started("toolu_1", "ls"));
    let start = Instant::now();

    live.heard(0, "ls", b"a\n", start);

    assert_eq!(live.heard(0, "ls", b"", start), None);
}
