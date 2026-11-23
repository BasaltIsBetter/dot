use std::os::unix::ffi::OsStringExt;

use super::*;

const SETUP: &str =
    "source /Users/me/.claude/shell-snapshots/snapshot-zsh-17-ab.sh 2>/dev/null || true";

fn tool_script(command: &str) -> String {
    format!("{SETUP} && eval '{command}' < /dev/null && pwd -P >| /tmp/claude-4122-cwd")
}

fn planned(script: &str, shell: Option<&str>) -> Plan {
    plan(vec![OsString::from(script)], shell.map(OsString::from))
}

fn handed(shell: &str, arguments: &[&str]) -> Plan {
    Plan::Hand {
        shell: PathBuf::from(shell),
        arguments: arguments.iter().map(OsString::from).collect(),
    }
}

#[test]
fn shortens_what_a_command_of_claude_prints() {
    let script = tool_script("cd crates && cargo test | tail -40");

    assert_eq!(
        planned(&script, Some("/bin/zsh")),
        Plan::Shorten {
            shell: PathBuf::from("/bin/zsh"),
            script: script.clone(),
            command: "cd crates && cargo test | tail -40".to_owned(),
        }
    );
}

#[test]
fn relays_the_contents_of_files_and_diffs_too() {
    for command in ["cat src/lib.rs", "git diff", "sed -n 1,40p src/lib.rs"] {
        let script = tool_script(command);

        assert_eq!(
            planned(&script, Some("/bin/zsh")),
            Plan::Shorten {
                shell: PathBuf::from("/bin/zsh"),
                script: script.clone(),
                command: command.to_owned(),
            },
            "{command}"
        );
    }
}

#[test]
fn hands_to_the_shell_what_must_stay_whole() {
    for command in [
        "DOT_RAW=1 cargo test",
        "DOT_RAW=1 cat src/lib.rs",
        "cat /var/folders/x/T/dot-whole/40000-42",
        "sed -n 171,582p /var/folders/x/T/dot-whole/40000-42",
    ] {
        let script = tool_script(command);

        assert_eq!(
            planned(&script, Some("/bin/zsh")),
            handed("/bin/zsh", &["-c", &script]),
            "{command}"
        );
    }
}

#[test]
fn hands_to_the_shell_what_is_not_a_command_of_claude() {
    for line in [
        "rtk hook claude",
        "node /Users/me/server.mjs --root /Users/me/Documents",
        "~/.claude/statusline.sh",
    ] {
        assert_eq!(
            planned(line, Some("/bin/zsh")),
            handed("/bin/zsh", &["-c", line]),
            "{line}"
        );
    }
}

#[test]
fn hands_over_whatever_else_it_is_given() {
    let not_text = OsString::from_vec(vec![b'l', b's', 0xff]);

    assert_eq!(
        plan(Vec::new(), Some(OsString::from("/bin/zsh"))),
        handed("/bin/zsh", &[])
    );
    assert_eq!(
        plan(
            vec![OsString::from("echo $1"), OsString::from("hello")],
            None
        ),
        handed("/bin/zsh", &["-c", "echo $1", "dot-shell", "hello"])
    );
    assert_eq!(
        plan(vec![not_text.clone()], None),
        Plan::Hand {
            shell: PathBuf::from("/bin/zsh"),
            arguments: vec![OsString::from("-c"), not_text],
        }
    );
    assert_eq!(
        plan(
            vec![
                OsString::from(tool_script("cargo test")),
                OsString::from("extra")
            ],
            None
        ),
        handed(
            "/bin/zsh",
            &["-c", &tool_script("cargo test"), "dot-shell", "extra"]
        )
    );
}

#[test]
fn runs_the_shell_the_command_line_was_written_for() {
    let zsh = tool_script("cargo test");
    let bash = zsh.replace("snapshot-zsh-", "snapshot-bash-");
    let shell_of = |script: &str, named: Option<&str>| match planned(script, named) {
        Plan::Shorten { shell, .. } | Plan::Hand { shell, .. } => shell,
    };

    assert_eq!(
        shell_of(&zsh, Some("/opt/homebrew/bin/zsh")),
        PathBuf::from("/opt/homebrew/bin/zsh")
    );
    assert_eq!(
        shell_of(&bash, Some("/opt/homebrew/bin/bash")),
        PathBuf::from("/opt/homebrew/bin/bash")
    );
    assert_eq!(
        shell_of(&bash, Some("/bin/zsh")),
        PathBuf::from("/bin/bash")
    );
    assert_eq!(shell_of(&zsh, Some("/bin/bash")), PathBuf::from("/bin/zsh"));
    assert_eq!(
        shell_of(&zsh, Some("/usr/local/bin/fish")),
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(
        shell_of(&bash, Some("/usr/local/bin/fish")),
        PathBuf::from("/bin/bash")
    );
    assert_eq!(shell_of(&zsh, Some("zsh")), PathBuf::from("/bin/zsh"));
    assert_eq!(shell_of(&zsh, None), PathBuf::from("/bin/zsh"));
    assert_eq!(
        shell_of("rtk hook claude", Some("/opt/homebrew/bin/bash")),
        PathBuf::from("/opt/homebrew/bin/bash")
    );
    assert_eq!(
        shell_of("rtk hook claude", Some("/usr/local/bin/fish")),
        PathBuf::from("/bin/zsh")
    );
}
