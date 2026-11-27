use std::fs::{File, OpenOptions, TryLockError};
use std::path::PathBuf;
use std::sync::OnceLock;

use dot_api::ArchiveError;

use crate::private::{create_private_folder, open_private_file};

const LOCK: &str = "lock";
const NO_FOLDER: &str = "Dot has no folder to keep its sessions in";
const ANOTHER_DOT: &str = "another Dot keeps the sessions";

/// The right of this Dot to keep the sessions: a lock on a file of their folder, taken the first
/// time it is needed and held until Dot stops, so that two Dots never write the same journal.
#[derive(Debug)]
pub(crate) struct Claim {
    folder: Option<PathBuf>,
    lock: OnceLock<Result<File, String>>,
}

impl Claim {
    pub(crate) const fn new(folder: Option<PathBuf>) -> Self {
        Self {
            folder,
            lock: OnceLock::new(),
        }
    }

    pub(crate) fn ours(&self) -> Result<(), ArchiveError> {
        match self.lock.get_or_init(|| self.take()) {
            Ok(_) => Ok(()),
            Err(refused) => Err(ArchiveError(refused.clone())),
        }
    }

    pub(crate) fn no_folder() -> ArchiveError {
        ArchiveError(NO_FOLDER.to_owned())
    }

    fn take(&self) -> Result<File, String> {
        let folder = self.folder.as_ref().ok_or(NO_FOLDER)?;
        create_private_folder(folder).map_err(|error| error.to_string())?;
        let file = open_private_file(
            &folder.join(LOCK),
            OpenOptions::new().write(true).create(true).truncate(false),
        )
        .map_err(|error| error.to_string())?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(TryLockError::WouldBlock) => Err(ANOTHER_DOT.to_owned()),
            Err(TryLockError::Error(error)) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/archive/claim.rs"]
mod tests;
