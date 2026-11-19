use std::fs;
use std::path::{Path, PathBuf};

use dot_api::{Effort, Limits, Model, Session, SessionStatus, Usage};

use crate::git;
use crate::readable::is_readable;

const UNREADABLE_CHARACTER: char = char::REPLACEMENT_CHARACTER;

/// Session in a folder of this Mac, that has sent nothing to Claude yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalSession {
    model: Model,
    effort: Effort,
    folder: Option<PathBuf>,
}

impl LocalSession {
    /// Creates a session that talks to `model` with `effort`, in `folder` when there is one.
    ///
    /// Nothing is read until [`Session::status`].
    #[must_use]
    pub const fn new(model: Model, effort: Effort, folder: Option<PathBuf>) -> Self {
        Self {
            model,
            effort,
            folder,
        }
    }
}

impl Session for LocalSession {
    fn status(&self) -> SessionStatus {
        let root = self.folder.as_deref().and_then(existing_folder);

        SessionStatus {
            model: self.model,
            effort: self.effort,
            project: root.as_deref().and_then(folder_name),
            branch: root.as_deref().and_then(git::branch),
            usage: Usage::default(),
            limits: Limits::default(),
        }
    }
}

fn existing_folder(folder: &Path) -> Option<PathBuf> {
    let root = fs::canonicalize(folder).ok()?;
    root.is_dir().then_some(root)
}

fn folder_name(root: &Path) -> Option<String> {
    let name = root.file_name()?.to_string_lossy();
    Some(
        name.chars()
            .map(|character| {
                if is_readable(character) {
                    character
                } else {
                    UNREADABLE_CHARACTER
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "../../../tests/dot-session/local.rs"]
mod tests;
