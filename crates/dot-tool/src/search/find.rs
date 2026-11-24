use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dot_api::{Cancellation, Detail, ToolOutput};

use crate::search::{count, glob, walk};
use crate::text::append;

const MOST_FILES: usize = 200;
const SEPARATOR: &str = "/";

pub(crate) fn find(root: &Path, pattern: &str, cancellation: &Cancellation) -> ToolOutput {
    let pattern = relative_pattern(root, pattern);
    let matcher = match glob(pattern) {
        Ok(matcher) => matcher,
        Err(reason) => return ToolOutput::failed(reason),
    };
    if !root.is_dir() {
        return ToolOutput::failed(format!("{} is not a folder.", root.display()));
    }

    let mut found: BinaryHeap<(Reverse<SystemTime>, PathBuf)> = BinaryHeap::new();
    let mut total = 0_usize;
    for entry in walk(root).filter_map(Result::ok) {
        if cancellation.is_cancelled() {
            return ToolOutput::stopped();
        }
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        if matcher.is_match(relative) {
            let modified = entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            total += 1;
            found.push((Reverse(modified), relative.to_path_buf()));
            if found.len() > MOST_FILES {
                found.pop();
            }
        }
    }

    if total == 0 {
        return ToolOutput::done(
            format!("No file under {} matches {pattern}.", root.display()),
            Detail::Files(0),
        );
    }
    let listed: Vec<String> = found
        .into_sorted_vec()
        .iter()
        .map(|(_, path)| path.display().to_string())
        .collect();
    let mut text = format!(
        "Files under {} that match {pattern}, newest first:\n{}",
        root.display(),
        listed.join("\n")
    );
    if total > MOST_FILES {
        let more = total - MOST_FILES;
        append(
            &mut text,
            format_args!("\n[{more} more files match: narrow the pattern to see them.]"),
        );
    }
    ToolOutput::done(text, Detail::Files(count(total)))
}

fn relative_pattern<'a>(root: &Path, pattern: &'a str) -> &'a str {
    let root = root.display().to_string();
    pattern
        .strip_prefix(root.as_str())
        .and_then(|rest| rest.strip_prefix(SEPARATOR))
        .unwrap_or(pattern)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/search/find.rs"]
mod tests;
