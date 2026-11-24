use std::borrow::Cow;
use std::fs;
use std::path::Path;

use dot_api::{Detail, FileChange, ToolOutput};
use dot_process::line_count;

use crate::file::store::{Original, failure, read_original, read_text, write_atomically};
use crate::file::uses_crlf;

const LF: &str = "\n";
const CRLF: &str = "\r\n";

pub(crate) fn create(path: &Path, text: &str) -> ToolOutput {
    run(|| {
        if path.is_dir() {
            return Err(format!("{} is a folder, not a file.", path.display()));
        }
        let (before, permissions, text) = match read_original(path)? {
            Some(Original { bytes, permissions }) => {
                let text = in_endings(text, uses_crlf(&bytes));
                (Some(into_text(bytes)), permissions, text)
            }
            None => (None, None, Cow::Borrowed(text)),
        };
        let removed = before
            .as_deref()
            .map_or(0, |before| line_count(before.as_bytes()));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| failure(parent, &error))?;
        }
        write_atomically(path, text.as_bytes(), permissions)
            .map_err(|error| failure(path, &error))?;
        let added = line_count(text.as_bytes());
        let change = FileChange::between(
            &path.display().to_string(),
            before.as_deref().unwrap_or_default(),
            &text,
            before.is_none(),
        );
        Ok(ToolOutput::done(
            format!("Wrote {added} lines to {}.", path.display()),
            Detail::Changes { added, removed },
        )
        .changing(change))
    })
}

pub(crate) fn replace(path: &Path, old: &str, new: &str) -> ToolOutput {
    run(|| {
        if old.is_empty() {
            return Err("The text to replace is empty: give the exact text to find.".to_owned());
        }
        let (text, original) = read_text(path)?;
        let crlf = uses_crlf(text.as_bytes());
        let old = in_endings(old, crlf);
        let new = in_endings(new, crlf);
        match text.matches(old.as_ref()).count() {
            0 => {
                return Err(format!(
                    "The text to replace is not in {}: view the file and copy the text exactly, \
                     spaces and line breaks included.",
                    path.display()
                ));
            }
            1 => {}
            found => {
                return Err(format!(
                    "The text to replace is in {} {found} times: add the lines around it so that \
                     it appears once.",
                    path.display()
                ));
            }
        }
        let changed = text.replacen(old.as_ref(), &new, 1);
        write_atomically(path, changed.as_bytes(), original.permissions)
            .map_err(|error| failure(path, &error))?;
        let change = FileChange::between(&path.display().to_string(), &text, &changed, false);
        Ok(ToolOutput::done(
            format!("Replaced the text in {}.", path.display()),
            Detail::Changes {
                added: line_count(new.as_bytes()),
                removed: line_count(old.as_bytes()),
            },
        )
        .changing(change))
    })
}

pub(crate) fn insert(path: &Path, after_line: u64, text: &str) -> ToolOutput {
    run(|| {
        let (content, original) = read_text(path)?;
        let total = line_count(content.as_bytes());
        if after_line > total {
            return Err(format!(
                "{} has {total} lines: there is no line {after_line} to insert after.",
                path.display()
            ));
        }
        let crlf = uses_crlf(content.as_bytes());
        let ending = if crlf { CRLF } else { LF };
        let inserted = in_endings(trim_one_ending(text), crlf);
        let offset = offset_after(&content, after_line);
        let block = if offset == content.len() && !content.is_empty() && !content.ends_with(LF) {
            format!("{ending}{inserted}")
        } else {
            format!("{inserted}{ending}")
        };
        let mut changed = String::with_capacity(content.len() + block.len());
        changed.push_str(&content[..offset]);
        changed.push_str(&block);
        changed.push_str(&content[offset..]);
        write_atomically(path, changed.as_bytes(), original.permissions)
            .map_err(|error| failure(path, &error))?;
        let change = FileChange::between(&path.display().to_string(), &content, &changed, false);
        Ok(ToolOutput::done(
            format!(
                "Inserted the text after line {after_line} of {}.",
                path.display()
            ),
            Detail::Changes {
                added: line_count(inserted.as_bytes()).max(1),
                removed: 0,
            },
        )
        .changing(change))
    })
}

fn run(edit: impl FnOnce() -> Result<ToolOutput, String>) -> ToolOutput {
    edit().unwrap_or_else(ToolOutput::failed)
}

fn in_endings(text: &str, crlf: bool) -> Cow<'_, str> {
    if !crlf && !text.contains(CRLF) {
        return Cow::Borrowed(text);
    }
    let unified = text.replace(CRLF, LF);
    if crlf {
        Cow::Owned(unified.replace(LF, CRLF))
    } else {
        Cow::Owned(unified)
    }
}

fn into_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

fn trim_one_ending(text: &str) -> &str {
    text.strip_suffix(CRLF)
        .or_else(|| text.strip_suffix(LF))
        .unwrap_or(text)
}

fn offset_after(content: &str, line: u64) -> usize {
    if line == 0 {
        return 0;
    }
    content
        .match_indices(LF)
        .nth(usize::try_from(line - 1).unwrap_or(usize::MAX))
        .map_or(content.len(), |(at, _)| at + 1)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/edit.rs"]
mod tests;
