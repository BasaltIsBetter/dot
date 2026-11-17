use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;

use dot_api::{ConnectError, ConnectFailure};

const CLAUDE_CODE: &str = "claude-code";
const LONGEST_CHOICE: u64 = 64;

pub(crate) struct ChoiceFile {
    path: PathBuf,
}

impl ChoiceFile {
    pub(crate) const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn holds_claude_code(&self) -> Result<bool, ConnectError> {
        let mut contents = String::new();
        match File::open(&self.path) {
            Ok(file) => file
                .take(LONGEST_CHOICE)
                .read_to_string(&mut contents)
                .map_err(storage_failed)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(storage_failed(error)),
        };
        Ok(contents == CLAUDE_CODE)
    }

    pub(crate) fn remember_claude_code(&self) -> Result<(), ConnectError> {
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder).map_err(storage_failed)?;
        }
        fs::write(&self.path, CLAUDE_CODE).map_err(storage_failed)
    }

    pub(crate) fn forget(&self) -> Result<(), ConnectError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(storage_failed(error)),
        }
    }
}

fn storage_failed(error: io::Error) -> ConnectError {
    ConnectError::caused_by(ConnectFailure::StorageFailed, error)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/choice.rs"]
pub(crate) mod tests;
