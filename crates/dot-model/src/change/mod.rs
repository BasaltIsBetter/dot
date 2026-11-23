use std::fs;
use std::path::{Path, PathBuf};

use dot_api::{FileChange, Hunk, PatchLine, Siblings};
use serde_json::Value;

use crate::tool::relative;

const NEIGHBOURS: usize = 2;
const CREATE: &str = "create";
const FOLDER_MARK: char = '/';
const HIDDEN: char = '.';
const SAME: char = ' ';
const REMOVED: char = '-';
const ADDED: char = '+';

pub(crate) fn as_shown(mut change: FileChange, folder: Option<&Path>) -> FileChange {
    if change.created {
        change.siblings = siblings_of(folder, &change.path);
    }
    change.path = relative(&change.path, folder);
    change
}

pub(crate) fn of_claude_code(result: &Value) -> Option<FileChange> {
    let path = result.get("filePath").and_then(Value::as_str)?;
    if let Some(Value::Array(hunks)) = result.get("structuredPatch")
        && !hunks.is_empty()
    {
        let change = FileChange {
            path: path.to_owned(),
            created: false,
            hunks: hunks.iter().map(hunk_of).collect(),
            siblings: None,
        };
        return Some(match result.get("originalFile").and_then(Value::as_str) {
            Some(original) => change.widened(original),
            None => change,
        });
    }
    let content = result.get("content").and_then(Value::as_str)?;
    let creates = result.get("type").and_then(Value::as_str) == Some(CREATE);
    creates.then(|| FileChange::between(path, "", content, true))
}

fn hunk_of(hunk: &Value) -> Hunk {
    let number = |key: &str| hunk.get(key).and_then(Value::as_u64).unwrap_or_default();
    let lines = hunk
        .get("lines")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |lines| {
            lines
                .iter()
                .filter_map(Value::as_str)
                .filter_map(line_of)
                .collect()
        });
    Hunk {
        old_start: number("oldStart"),
        new_start: number("newStart"),
        lines,
    }
}

fn line_of(line: &str) -> Option<PatchLine> {
    let mut characters = line.chars();
    let kind = characters.next()?;
    let text = characters.as_str().to_owned();
    match kind {
        SAME => Some(PatchLine::Same(text)),
        REMOVED => Some(PatchLine::Removed(text)),
        ADDED => Some(PatchLine::Added(text)),
        _ => None,
    }
}

fn siblings_of(folder: Option<&Path>, path: &str) -> Option<Siblings> {
    let file: PathBuf = folder.map_or_else(|| PathBuf::from(path), |folder| folder.join(path));
    let parent = file.parent()?;
    let name = file.file_name()?.to_string_lossy().into_owned();
    let mut entries: Vec<String> = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let shown = entry.file_name().to_string_lossy().into_owned();
            if shown.starts_with(HIDDEN) || shown == name {
                return None;
            }
            let is_folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            Some(if is_folder {
                format!("{shown}{FOLDER_MARK}")
            } else {
                shown
            })
        })
        .collect();
    entries.sort_by_key(|entry| entry.to_lowercase());
    let at = entries.partition_point(|entry| entry.to_lowercase() < name.to_lowercase());
    let folder_name = parent
        .file_name()
        .map_or_else(String::new, |folder| folder.to_string_lossy().into_owned());
    Some(Siblings {
        folder: format!("{folder_name}{FOLDER_MARK}"),
        before: entries[at.saturating_sub(NEIGHBOURS)..at].to_vec(),
        after: entries[at..(at + NEIGHBOURS).min(entries.len())].to_vec(),
    })
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/change/mod.rs"]
mod tests;
