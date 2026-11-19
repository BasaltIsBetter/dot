use std::io::Read;
use std::path::{Path, PathBuf};

use dot_api::{Cancellation, Detail, SearchShow, TextSearch, ToolOutput};
use regex::bytes::{Regex, RegexBuilder};

use crate::file::{SNIFFED, is_binary, open_without_waiting};
use crate::search::{count, glob, matches_name_or_path, walk};
use crate::text::append;

const MOST_FILES: usize = 200;
const MOST_LINES: usize = 500;
const LONGEST_LINE: usize = 500;
const LARGEST_SEARCHED: u64 = 10 * 1_024 * 1_024;
const NEW_LINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const CUT_MARK: &str = " […]";

struct Found {
    path: PathBuf,
    lines: Vec<(usize, String)>,
    count: usize,
}

pub(crate) fn search(root: &Path, search: &TextSearch, cancellation: &Cancellation) -> ToolOutput {
    let regex = match RegexBuilder::new(&search.pattern)
        .case_insensitive(search.ignore_case)
        .build()
    {
        Ok(regex) => regex,
        Err(error) => {
            return ToolOutput::failed(format!("The regular expression is not valid: {error}."));
        }
    };
    let filter = match search.files.as_deref().map(glob).transpose() {
        Ok(filter) => filter,
        Err(reason) => return ToolOutput::failed(reason),
    };

    let mut found = Vec::new();
    let mut lines_kept = 0;
    let mut buffer = Vec::new();
    for entry in walk(root).filter_map(Result::ok) {
        if cancellation.is_cancelled() {
            return ToolOutput::stopped();
        }
        if entry.path_is_symlink() || !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let relative = shown_path(root, entry.path());
        let filtered_out = filter
            .as_ref()
            .zip(search.files.as_deref())
            .is_some_and(|(matcher, pattern)| !matches_name_or_path(matcher, pattern, &relative));
        if filtered_out || !read(entry.path(), &mut buffer) {
            continue;
        }
        let keep = if search.show == SearchShow::Lines {
            MOST_LINES - lines_kept
        } else {
            0
        };
        if let Some(file) = matching(&regex, &buffer, relative, keep) {
            lines_kept += file.lines.len();
            found.push(file);
        }
    }
    answer(root, search, &found)
}

fn shown_path(root: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(relative) if !relative.as_os_str().is_empty() => relative.to_path_buf(),
        _ => PathBuf::from(path.file_name().unwrap_or(path.as_os_str())),
    }
}

fn read(path: &Path, buffer: &mut Vec<u8>) -> bool {
    buffer.clear();
    let Ok((file, metadata)) = open_without_waiting(path) else {
        return false;
    };
    let small = metadata.is_file() && metadata.len() <= LARGEST_SEARCHED;
    small
        && file.take(LARGEST_SEARCHED).read_to_end(buffer).is_ok()
        && !is_binary(&buffer[..buffer.len().min(usize::try_from(SNIFFED).unwrap_or(0))])
}

fn matching(regex: &Regex, bytes: &[u8], path: PathBuf, keep: usize) -> Option<Found> {
    if !regex.is_match(bytes) {
        return None;
    }
    let mut file = Found {
        path,
        lines: Vec::new(),
        count: 0,
    };
    for (index, line) in bytes.split(|&byte| byte == NEW_LINE).enumerate() {
        let line = line.strip_suffix(&[CARRIAGE_RETURN]).unwrap_or(line);
        if !regex.is_match(line) {
            continue;
        }
        file.count += 1;
        if file.lines.len() < keep {
            file.lines.push((index + 1, shown_line(line)));
        }
    }
    Some(file).filter(|file| file.count > 0)
}

fn shown_line(line: &[u8]) -> String {
    let line = String::from_utf8_lossy(line);
    let mut kept: String = line.chars().take(LONGEST_LINE).collect();
    if kept.len() < line.len() {
        kept.push_str(CUT_MARK);
    }
    kept
}

fn answer(root: &Path, search: &TextSearch, found: &[Found]) -> ToolOutput {
    let matches: usize = found.iter().map(|file| file.count).sum();
    if found.is_empty() {
        return ToolOutput::done(
            format!(
                "Nothing under {} matches {}.",
                root.display(),
                search.pattern
            ),
            detail(search.show, 0, 0),
        );
    }
    let mut text = String::new();
    match search.show {
        SearchShow::Files => {
            for file in found.iter().take(MOST_FILES) {
                append(&mut text, format_args!("{}\n", file.path.display()));
            }
            note(&mut text, found.len() > MOST_FILES, "files", MOST_FILES);
        }
        SearchShow::Counts => {
            for file in found.iter().take(MOST_FILES) {
                append(
                    &mut text,
                    format_args!("{}:{}\n", file.path.display(), file.count),
                );
            }
            note(&mut text, found.len() > MOST_FILES, "files", MOST_FILES);
        }
        SearchShow::Lines => {
            for file in found {
                for (number, line) in &file.lines {
                    append(
                        &mut text,
                        format_args!("{}:{number}:{line}\n", file.path.display()),
                    );
                }
            }
            note(&mut text, matches > MOST_LINES, "lines", MOST_LINES);
        }
    }
    ToolOutput::done(text, detail(search.show, found.len(), matches))
}

fn note(text: &mut String, cut: bool, what: &str, most: usize) {
    if cut {
        append(
            text,
            format_args!(
                "[Only the first {most} {what} are shown: narrow the search to see the rest.]"
            ),
        );
    }
}

fn detail(show: SearchShow, files: usize, matches: usize) -> Detail {
    match show {
        SearchShow::Files => Detail::Files(count(files)),
        SearchShow::Lines | SearchShow::Counts => Detail::Matches(count(matches)),
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/search/text.rs"]
mod tests;
