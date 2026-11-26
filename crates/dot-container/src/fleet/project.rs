use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const UNSAFE_IN_A_MOUNT: [char; 3] = [',', '"', ':'];

/// The folder an agent works in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project(PathBuf);

/// Why a folder cannot be handed to an agent.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// It cannot be found.
    #[error("{}", .0.display())]
    Missing(PathBuf, #[source] io::Error),
    /// It holds far more than a project: an agent must not see the user's whole home or disk.
    #[error("{} is no project: an agent would see far too much", .0.display())]
    TooWide(PathBuf),
    /// Its path cannot be written in a mount.
    #[error("the path {} cannot be mounted in a container", .0.display())]
    Unmountable(PathBuf),
}

impl Project {
    /// Takes `folder` as a project, links followed, for a user whose home is `home`.
    ///
    /// # Errors
    ///
    /// Returns an error if `folder` is not a folder, if it is the root of the disk, the user's
    /// home or a folder around it, or if its path cannot be written in a mount.
    pub fn new(folder: &Path, home: Option<&Path>) -> Result<Self, ProjectError> {
        let missing = |error| ProjectError::Missing(folder.to_owned(), error);
        let resolved = fs::canonicalize(folder).map_err(missing)?;
        if !resolved.is_dir() {
            return Err(missing(io::ErrorKind::NotADirectory.into()));
        }
        let home = home.and_then(|home| fs::canonicalize(home).ok());
        let holds_the_home = home.is_some_and(|home| home.starts_with(&resolved));
        if resolved.parent().is_none() || holds_the_home {
            return Err(ProjectError::TooWide(resolved));
        }
        let mountable = resolved.to_str().is_some_and(|path| {
            !path.contains(UNSAFE_IN_A_MOUNT) && !path.chars().any(char::is_control)
        });
        if !mountable {
            return Err(ProjectError::Unmountable(resolved));
        }
        Ok(Self(resolved))
    }

    /// The path of the folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn text(&self) -> String {
        self.0.display().to_string()
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/fleet/project.rs"]
mod tests;
