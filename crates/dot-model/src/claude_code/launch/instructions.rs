use std::fs;
use std::io::Read;
use std::path::Path;

use serde_json::json;

const INSTRUCTIONS_FILE: &str = "CLAUDE.md";
#[cfg(target_os = "macos")]
pub(crate) const LARGEST_INSTRUCTIONS: u64 = 256 * 1_024;
/// Linux takes 128 KiB at most in one argument, where the instructions follow Dot's own note.
#[cfg(target_os = "linux")]
pub(crate) const LARGEST_INSTRUCTIONS: u64 = 120 * 1_024;
const UNSAFE_IN_A_PATTERN_OR_A_SHELL: &[char] = &[
    '*', '?', '[', ']', '{', '}', '(', ')', '|', '!', '\'', '"', '\\',
];
const FENCES: [&str; 2] = ["```", "~~~"];
const CODE_SPAN: char = '`';
const IMPORT: char = '@';
const SUBAGENT_START: &str = "SubagentStart";
const PRINT: &str = "/usr/bin/printf %s";
const NUL: char = '\0';

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Handed {
    pub(crate) note: String,
    pub(crate) settings: String,
}

pub(crate) fn handed(folder: &Path) -> Option<Handed> {
    let file = folder.join(INSTRUCTIONS_FILE);
    let path = file.to_str().filter(|path| is_safe_to_name(path))?;
    let text = read(&file)?;
    if imports_another_file(&text) {
        return None;
    }
    let resolved = fs::canonicalize(&file).ok()?;
    let resolved = resolved.to_str().filter(|path| is_safe_to_name(path))?;
    let mut excluded = vec![path];
    if resolved != path {
        excluded.push(resolved);
    }

    let told_to_agents = json!({
        "hookSpecificOutput": {
            "hookEventName": SUBAGENT_START,
            "additionalContext": format!(
                "Before anything else, read the whole file {path}, in several reads if it is \
                 long: it holds the rules of this project, which you must follow."
            ),
        }
    });
    let settings = json!({
        "claudeMdExcludes": excluded,
        "hooks": {
            SUBAGENT_START: [{
                "hooks": [{"type": "command", "command": format!("{PRINT} '{told_to_agents}'")}],
            }],
        },
    });
    Some(Handed {
        note: format!(
            "\n\nContents of {path} (project instructions, checked into the codebase):\n\n{text}"
        ),
        settings: settings.to_string(),
    })
}

fn is_safe_to_name(path: &str) -> bool {
    Path::new(path).is_absolute()
        && !path.chars().any(|character| {
            character.is_control() || UNSAFE_IN_A_PATTERN_OR_A_SHELL.contains(&character)
        })
}

fn read(file: &Path) -> Option<String> {
    let is_a_plain_file = fs::symlink_metadata(file).is_ok_and(|metadata| metadata.is_file());
    if !is_a_plain_file {
        return None;
    }
    let (opened, metadata) = dot_process::open_without_waiting(file).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mut written = Vec::new();
    opened
        .take(LARGEST_INSTRUCTIONS + 1)
        .read_to_end(&mut written)
        .ok()?;
    if u64::try_from(written.len()).is_ok_and(|length| length > LARGEST_INSTRUCTIONS) {
        return None;
    }
    String::from_utf8(written)
        .ok()
        .filter(|text| !text.contains(NUL))
}

fn imports_another_file(text: &str) -> bool {
    let mut fenced = false;
    text.lines().any(|line| {
        if FENCES
            .iter()
            .any(|fence| line.trim_start().starts_with(fence))
        {
            fenced = !fenced;
            return false;
        }
        !fenced && names_an_import(line)
    })
}

fn names_an_import(line: &str) -> bool {
    let mut in_code = false;
    let mut starts_a_word = true;
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == CODE_SPAN {
            in_code = !in_code;
        }
        let follows = characters.peek().copied();
        let opens_an_import = character == IMPORT
            && starts_a_word
            && follows.is_some_and(|next| !next.is_whitespace());
        if !in_code && opens_an_import {
            return true;
        }
        starts_a_word = character.is_whitespace();
    }
    false
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/instructions.rs"]
mod tests;
