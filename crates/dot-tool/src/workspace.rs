use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use dot_api::{LARGEST_EDITED, Workspace, WorkspaceError};

use crate::file::open_without_waiting;
use crate::file::store::{read_original, write_atomically};
use crate::file::uses_crlf;
use crate::place::{locate, resolve};

const CRLF: &str = "\r\n";
const LF: &str = "\n";
const HIDDEN: char = '.';
const FOLDER_MARK: char = '/';
const ITSELF: &str = ".";

/// [`Workspace`] of a session's folder on this Mac, and of the files Claude changed elsewhere.
///
/// Paths are resolved once, links followed. A relative path is refused when it leads out of the
/// folder; an absolute path is taken wherever it leads. Writing goes through a temporary file
/// renamed over the old one, so a file is never left half written.
pub struct LocalWorkspace {
    folder: Option<PathBuf>,
}

impl LocalWorkspace {
    /// Creates the workspace of a session in `folder`, or of a session without a folder, where
    /// only absolute paths are taken.
    ///
    /// Nothing is read until a file is asked for.
    #[must_use]
    pub const fn new(folder: Option<PathBuf>) -> Self {
        Self { folder }
    }

    fn resolved(&self, path: &str) -> Result<PathBuf, WorkspaceError> {
        if Path::new(path).is_absolute() {
            return resolve(None, path).map_err(|error| WorkspaceError::Failed(error.to_string()));
        }
        let root = self
            .folder
            .as_deref()
            .and_then(|folder| fs::canonicalize(folder).ok())
            .ok_or(WorkspaceError::Outside)?;
        let asked = if path.is_empty() { ITSELF } else { path };
        let place = locate(Some(&root), asked).map_err(|_| WorkspaceError::Outside)?;
        if !place.inside {
            return Err(WorkspaceError::Outside);
        }
        Ok(place.path)
    }
}

impl Workspace for LocalWorkspace {
    fn read(&self, path: &str) -> Result<String, WorkspaceError> {
        let path = self.resolved(path)?;
        let (file, metadata) = open_without_waiting(&path).map_err(|error| failed(&error))?;
        if !metadata.is_file() {
            return Err(WorkspaceError::NotAFile);
        }
        let mut bytes = Vec::new();
        file.take(LARGEST_EDITED + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| failed(&error))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > LARGEST_EDITED {
            return Err(WorkspaceError::TooLarge);
        }
        let text = String::from_utf8(bytes).map_err(|_| WorkspaceError::NotText)?;
        Ok(text.replace(CRLF, LF))
    }

    fn write(&self, path: &str, text: &str) -> Result<(), WorkspaceError> {
        let path = self.resolved(path)?;
        if path.is_dir() {
            return Err(WorkspaceError::NotAFile);
        }
        let original = read_original(&path).map_err(WorkspaceError::Failed)?;
        let crlf = original
            .as_ref()
            .is_some_and(|original| uses_crlf(&original.bytes));
        let text = if crlf {
            text.replace(LF, CRLF)
        } else {
            text.to_owned()
        };
        let permissions = original.and_then(|original| original.permissions);
        write_atomically(&path, text.as_bytes(), permissions).map_err(|error| failed(&error))
    }

    fn list(&self, path: &str) -> Result<Vec<String>, WorkspaceError> {
        let path = self.resolved(path)?;
        let mut names: Vec<String> = fs::read_dir(&path)
            .map_err(|error| failed(&error))?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(HIDDEN) {
                    return None;
                }
                let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
                Some(if folder {
                    format!("{name}{FOLDER_MARK}")
                } else {
                    name
                })
            })
            .collect();
        names.sort_by_cached_key(|name| name.to_lowercase());
        Ok(names)
    }
}

fn failed(error: &io::Error) -> WorkspaceError {
    WorkspaceError::Failed(error.to_string())
}

#[cfg(test)]
#[path = "../../../tests/dot-tool/workspace.rs"]
mod tests;
