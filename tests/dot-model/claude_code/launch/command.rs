use std::ffi::OsStr;
use std::path::PathBuf;

use super::*;

const SESSION: &str = "3898f4c9-b1b3-480e-b180-b139f7eb2054";

fn arguments(command: &Command) -> Vec<&str> {
    command
        .get_args()
        .map(|argument| argument.to_str().unwrap())
        .collect()
}

fn launch(session: Option<&SessionId>) -> Command {
    command(
        Path::new("/usr/local/bin/claude"),
        Path::new("/Users/someone/Library/Application Support/Dot"),
        "claude-opus-5-5",
        Effort::High,
        session,
    )
}

#[test]
fn reads_a_session_id_shaped_like_claude_code_s() {
    assert!(SessionId::parse(SESSION).is_some());
}

#[test]
fn refuses_a_session_id_that_could_pass_for_an_option() {
    assert_eq!(
        SessionId::parse("--dangerously-skip-permissions-xxxxxx"),
        None
    );
    assert_eq!(
        SessionId::parse("-898f4c9-b1b3-480e-b180-b139f7eb2054"),
        None
    );
}

#[test]
fn refuses_a_session_id_of_the_wrong_shape() {
    assert_eq!(SessionId::parse(&SESSION[1..]), None);
    assert_eq!(SessionId::parse(&format!("{SESSION}0")), None);
    assert_eq!(
        SessionId::parse("3898f4c9-b1b3-480e-b180-b139f7eb205z"),
        None
    );
    assert_eq!(
        SessionId::parse("3898f4c9 b1b3-480e-b180-b139f7eb2054"),
        None
    );
    assert_eq!(SessionId::parse(""), None);
}

#[test]
fn starts_claude_code_streaming_and_asking_dot_for_every_permission() {
    let command = launch(None);

    assert_eq!(command.get_program(), OsStr::new("/usr/local/bin/claude"));
    assert_eq!(
        arguments(&command),
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--permission-mode",
            "manual",
            "--permission-prompt-tool",
            "stdio",
            "--effort",
            "high",
            "--model",
            "claude-opus-5-5",
            "--append-system-prompt",
            CLAUDE_CODE_NOTE,
        ]
    );
}

#[test]
fn resumes_the_session_it_was_given() {
    let session = SessionId::parse(SESSION).unwrap();

    let command = launch(Some(&session));

    assert!(arguments(&command).ends_with(&["--resume", SESSION]));
}

#[test]
fn never_restricts_claude_code() {
    let command = launch(None);
    let arguments = arguments(&command);

    for restriction in [
        "--restricted",
        "--safe-mode",
        "--bare",
        "--disable-slash-commands",
        "--strict-mcp-config",
        "--setting-sources",
        "--tools",
        "--disallowedTools",
    ] {
        assert!(!arguments.contains(&restriction), "{restriction}");
    }
}

#[test]
fn runs_in_the_folder_it_was_given_without_anthropic_credentials() {
    let command = launch(None);

    assert_eq!(
        command.get_current_dir(),
        Some(Path::new("/Users/someone/Library/Application Support/Dot"))
    );
    let removed: Vec<PathBuf> = command
        .get_envs()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| PathBuf::from(name))
        .collect();
    assert_eq!(
        removed,
        [
            PathBuf::from("ANTHROPIC_API_KEY"),
            PathBuf::from("ANTHROPIC_AUTH_TOKEN")
        ]
    );
}
