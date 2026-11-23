use std::fs::File;
use std::io::Read;
use std::path::Path;

use dot_api::Summaries;
use serde_json::Value;

const SETTINGS_FILES: [&str; 2] = [".claude/settings.json", ".claude/settings.local.json"];
const WINDOW_SETTING: &str = "autoCompactWindow";
const LARGEST_SETTINGS_FILE: u64 = 1_024 * 1_024;

/// Returns `wanted`, unless the user chose in Claude Code's settings the size at which Claude
/// Code summarizes a conversation: then [`Summaries::LeftToClaude`], so that Dot never replaces
/// that choice with a size of its own.
///
/// Looks for `autoCompactWindow`, for every model or for one, in `.claude/settings.json` and
/// `.claude/settings.local.json` under each of `folders`: the user's home and the project.
/// Reads at most 1 MiB of each file; a file that is missing, unreadable or not JSON sets
/// nothing.
#[must_use]
pub fn summaries_for(wanted: Summaries, folders: &[&Path]) -> Summaries {
    let chosen_by_the_user = folders
        .iter()
        .flat_map(|folder| SETTINGS_FILES.map(|file| folder.join(file)))
        .any(|file| sets_the_window(&file));
    if chosen_by_the_user {
        return Summaries::LeftToClaude;
    }
    wanted
}

fn sets_the_window(settings: &Path) -> bool {
    let mut written = Vec::new();
    let read = File::open(settings)
        .map(|file| file.take(LARGEST_SETTINGS_FILE))
        .and_then(|mut file| file.read_to_end(&mut written));
    read.is_ok() && serde_json::from_slice(&written).is_ok_and(|value| names_the_window(&value))
}

fn names_the_window(value: &Value) -> bool {
    match value {
        Value::Object(fields) => fields
            .iter()
            .any(|(name, value)| name == WINDOW_SETTING || names_the_window(value)),
        Value::Array(items) => items.iter().any(names_the_window),
        _ => false,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/settings.rs"]
mod tests;
