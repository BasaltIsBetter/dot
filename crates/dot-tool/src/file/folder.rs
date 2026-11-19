use std::fs;
use std::path::Path;

use dot_api::{Detail, ToolOutput};

use crate::text::append;

const MOST_ENTRIES: usize = 1_000;
const SKIPPED: [&str; 2] = ["node_modules", "target"];
const HIDDEN_MARK: char = '.';
const FOLDER_MARK: &str = "/";

struct Entry {
    name: String,
    is_folder: bool,
}

pub(crate) fn list(path: &Path) -> ToolOutput {
    let top = match entries(path) {
        Ok(top) => top,
        Err(error) => {
            return ToolOutput::failed(format!("Cannot list {}: {error}.", path.display()));
        }
    };
    let mut listed = Vec::new();
    let mut more = false;

    'top: for entry in top {
        let children = if entry.is_folder {
            entries(&path.join(&entry.name)).unwrap_or_default()
        } else {
            Vec::new()
        };
        let lines = std::iter::once(shown(&entry, None))
            .chain(children.iter().map(|child| shown(child, Some(&entry.name))));
        for line in lines {
            if listed.len() == MOST_ENTRIES {
                more = true;
                break 'top;
            }
            listed.push(line);
        }
    }

    let count = listed.len();
    let mut text = format!(
        "Entries of {}, two levels deep, hidden ones and {} left out:\n{}",
        path.display(),
        SKIPPED.join(", "),
        listed.join("\n")
    );
    if more {
        append(
            &mut text,
            format_args!(
                "\n[Only the first {MOST_ENTRIES} entries are shown: list a subfolder to see more.]"
            ),
        );
    }
    ToolOutput::done(
        text,
        Detail::Files(u64::try_from(count).unwrap_or(u64::MAX)),
    )
}

fn entries(path: &Path) -> std::io::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let kept = !name.starts_with(HIDDEN_MARK) && !SKIPPED.contains(&name.as_str());
            let is_folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            kept.then_some(Entry { name, is_folder })
        })
        .collect();
    entries.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn shown(entry: &Entry, parent: Option<&str>) -> String {
    let mark = if entry.is_folder { FOLDER_MARK } else { "" };
    match parent {
        Some(parent) => format!("{parent}{FOLDER_MARK}{}{mark}", entry.name),
        None => format!("{}{mark}", entry.name),
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/folder.rs"]
mod tests;
