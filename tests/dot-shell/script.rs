use proptest::prelude::*;

use super::*;

const SETUP: &str = "source /Users/me/.claude/shell-snapshots/snapshot-zsh-1795117850254-623hxs.sh 2>/dev/null || true && { \\builtin shopt -u extglob || \\builtin setopt NO_EXTENDED_GLOB NO_BARE_GLOB_QUAL NO_BRACE_CCL NO_MAGIC_EQUAL_SUBST NO_EQUALS; } >/dev/null 2>&1 || true && { \\builtin unalias -- 'which'; \\builtin unset -f -- 'which'; } >/dev/null 2>&1 || true";

fn tool_script(quoted: &str) -> String {
    format!("{SETUP} && eval {quoted} < /dev/null && pwd -P >| /tmp/claude-4122-cwd")
}

fn quoted(command: &str) -> String {
    format!("'{}'", command.replace('\'', "'\"'\"'"))
}

#[test]
fn finds_the_command_claude_wrote_in_what_claude_code_assembles() {
    assert_eq!(
        asked(&tool_script("'echo RAW_OK; echo RAW_ERR 1>&2'")).as_deref(),
        Some("echo RAW_OK; echo RAW_ERR 1>&2")
    );
    assert_eq!(
        asked(&tool_script(r#"'sh -c "echo RAW_FAIL; exit 3"'"#)).as_deref(),
        Some(r#"sh -c "echo RAW_FAIL; exit 3""#)
    );
    assert_eq!(asked(&tool_script("pwd")).as_deref(), Some("pwd"));
}

#[test]
fn reads_every_way_a_shell_quotes_a_word() {
    assert_eq!(
        asked(&tool_script(r#"'echo '"'"'it'"'"''"#)).as_deref(),
        Some("echo 'it'")
    );
    assert_eq!(
        asked(&tool_script(r"'echo '\''it'\'''")).as_deref(),
        Some("echo 'it'")
    );
    assert_eq!(
        asked(&tool_script(r#""echo \"a\" \$HOME \n""#)).as_deref(),
        Some(r#"echo "a" $HOME \n"#)
    );
    assert_eq!(asked(&tool_script(r"echo\ a")).as_deref(), Some("echo a"));
}

#[test]
fn a_command_may_itself_contain_what_surrounds_it() {
    let command = "true && eval 'x' < /dev/null && pwd -P >| /tmp/mine";

    assert_eq!(
        asked(&tool_script(&quoted(command))).as_deref(),
        Some(command)
    );
}

#[test]
fn does_not_take_another_command_line_for_a_command_of_claude() {
    for script in [
        "rtk hook claude",
        "node /Users/me/server.mjs --root /Users/me/Documents",
        "",
        "pwd -P >| /tmp/claude-4122-cwd",
        "true && eval pwd && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval 'unterminated < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval pwd; rm -rf x < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval two words < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval '' < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval \"unterminated < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
        "true && eval end\\ < /dev/null && pwd -P >| /tmp/claude-4122-cwd",
    ] {
        assert_eq!(asked(script), None, "{script}");
    }
}

#[test]
fn tells_the_shell_a_command_line_was_written_for_by_its_snapshot() {
    assert_eq!(snapshot(&tool_script("pwd")), Some(Snapshot::Zsh));
    assert_eq!(
        snapshot("source /Users/me/.claude/shell-snapshots/snapshot-bash-17-ab.sh && eval pwd"),
        Some(Snapshot::Bash)
    );
    assert_eq!(snapshot("rtk hook claude"), None);
}

proptest! {
    #[test]
    fn any_command_is_found_again_however_it_is_quoted(command in "[^\\x00]{1,60}") {
        let single = format!("'{}'", command.replace('\'', r"'\''"));

        prop_assert_eq!(asked(&tool_script(&quoted(&command))), Some(command.clone()));
        prop_assert_eq!(asked(&tool_script(&single)), Some(command));
    }

    #[test]
    fn any_command_line_is_read_without_panicking(script in any::<String>()) {
        let _ = asked(&script);
        let _ = snapshot(&script);
    }
}
