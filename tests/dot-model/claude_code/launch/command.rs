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

const NOTHING_MORE: Options<'static> = Options {
    session: None,
    shell: None,
    compaction: OutputCompaction::FULL,
    live: None,
    path: None,
    summary_window: None,
    reading: Reading::ByClaude,
    instructions: None,
};

fn launched(options: Options<'_>) -> Command {
    command(
        Path::new("/usr/local/bin/claude"),
        Path::new("/Users/someone/Library/Application Support/Dot"),
        "claude-opus-5-5",
        Effort::High,
        options,
    )
}

fn launch(session: Option<&SessionId>) -> Command {
    launched(Options {
        session,
        ..NOTHING_MORE
    })
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

fn launched_with(shell: &str) -> Command {
    launched(Options {
        shell: Some(Path::new(shell)),
        ..NOTHING_MORE
    })
}

fn shell_prefix(command: &Command) -> Option<&OsStr> {
    command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new("CLAUDE_CODE_SHELL_PREFIX"))
        .and_then(|(_, value)| value)
}

fn compaction_told(command: &Command) -> Option<&OsStr> {
    command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new("DOT_COMPACTION"))
        .and_then(|(_, value)| value)
}

#[test]
fn tells_dot_s_shell_stand_in_how_to_shorten_outputs() {
    let compaction = OutputCompaction {
        progress: dot_api::Toggle::Off,
        outputs_cut: dot_api::CutLevel::Never,
        ..OutputCompaction::FULL
    };
    let command = launched(Options {
        shell: Some(Path::new("/Applications/Dot.app/Contents/MacOS/dot-shell")),
        compaction,
        ..NOTHING_MORE
    });

    let told = compaction_told(&command).and_then(OsStr::to_str).unwrap();
    assert_eq!(OutputCompaction::from_variable(told), Some(compaction));
}

#[test]
fn tells_nothing_about_outputs_without_a_shell_stand_in() {
    assert_eq!(compaction_told(&launch(None)), None);
    assert_eq!(
        compaction_told(&launched_with("/Users/some one/dot-shell")),
        None
    );
}

fn changes_the_path(command: &Command) -> bool {
    command
        .get_envs()
        .any(|(name, _)| name == OsStr::new("PATH"))
}

fn path_of(command: &Command) -> Option<&OsStr> {
    command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new("PATH"))
        .and_then(|(_, value)| value)
}

#[test]
fn hands_claude_code_the_path_it_is_given() {
    let command = launched(Options {
        path: Some(OsStr::new("/opt/homebrew/bin:/usr/bin:/bin")),
        ..NOTHING_MORE
    });

    assert_eq!(
        path_of(&command),
        Some(OsStr::new("/opt/homebrew/bin:/usr/bin:/bin"))
    );
}

#[test]
fn leaves_dot_s_own_path_to_claude_code_without_one() {
    assert!(!changes_the_path(&launch(None)));
}

#[test]
fn names_dot_s_shell_stand_in_to_claude_code() {
    let command = launched_with("/Applications/Dot.app/Contents/MacOS/dot-shell");

    assert_eq!(
        shell_prefix(&command),
        Some(OsStr::new("/Applications/Dot.app/Contents/MacOS/dot-shell"))
    );
}

#[test]
fn names_no_stand_in_when_there_is_none_or_its_path_would_not_survive_a_shell() {
    assert_eq!(shell_prefix(&launch(None)), None);
    for shell in [
        "/Users/some one/Dot.app/Contents/MacOS/dot-shell",
        "/Users/someone/it's/dot-shell",
        "/Users/someone/$HOME/dot-shell",
        "/Users/someone/a;b/dot-shell",
        "relative/dot-shell",
        "",
    ] {
        assert_eq!(shell_prefix(&launched_with(shell)), None, "{shell}");
    }
}
#[test]
fn hands_claude_code_the_summary_window_only_when_there_is_one() {
    let with = launched(Options {
        summary_window: Some(312_500),
        ..NOTHING_MORE
    });

    let arguments_with = arguments(&with);
    let at = arguments_with
        .iter()
        .position(|argument| *argument == "--autocompact");
    assert_eq!(at.map(|at| arguments_with[at + 1]), Some("312500"));
    assert!(!arguments(&launch(None)).contains(&"--autocompact"));
}

fn after<'a>(arguments: &[&'a str], option: &str) -> Option<&'a str> {
    let at = arguments.iter().position(|argument| *argument == option)?;
    arguments.get(at + 1).copied()
}

#[test]
fn hands_claude_code_dot_s_reader_and_says_when_to_use_it_only_when_asked_to() {
    let with = launched(Options {
        reading: Reading::ByACheaperAgent,
        ..NOTHING_MORE
    });
    let without = launch(None);

    let arguments_with = arguments(&with);
    assert_eq!(after(&arguments_with, "--agents"), Some(READER_AGENT));
    assert_eq!(
        after(&arguments_with, "--append-system-prompt"),
        Some(format!("{CLAUDE_CODE_NOTE}{READER_NOTE}").as_str())
    );
    assert_eq!(after(&arguments(&without), "--agents"), None);
    assert_eq!(
        after(&arguments(&without), "--append-system-prompt"),
        Some(CLAUDE_CODE_NOTE)
    );
}
#[test]
fn hands_the_project_s_instructions_over_after_the_note_with_their_settings() {
    let handed = Handed {
        note: "\n\nContents of /p/CLAUDE.md (project instructions, checked into the codebase):\n\nRules.".to_owned(),
        settings: r#"{"claudeMdExcludes":["/p/CLAUDE.md"]}"#.to_owned(),
    };
    let with = launched(Options {
        reading: Reading::ByACheaperAgent,
        instructions: Some(&handed),
        ..NOTHING_MORE
    });

    let arguments_with = arguments(&with);
    assert_eq!(
        after(&arguments_with, "--append-system-prompt"),
        Some(format!("{CLAUDE_CODE_NOTE}{READER_NOTE}{}", handed.note).as_str())
    );
    assert_eq!(
        after(&arguments_with, "--settings"),
        Some(handed.settings.as_str())
    );
    assert_eq!(after(&arguments(&launch(None)), "--settings"), None);
}
