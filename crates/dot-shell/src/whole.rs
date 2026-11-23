use std::ffi::OsString;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dot_api::WHOLE_OUTPUTS_FOLDER;

const KEPT_FOR: Duration = Duration::from_hours(24);
const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;
const FOLDER_OF_ITS_OWNER: u32 = 0o700;
const FILE_OF_ITS_OWNER: u32 = 0o600;
const OPEN_TO_OTHERS: u32 = 0o077;
const BYTES_IN_A_MEBIBYTE: usize = 1_024 * 1_024;
const THE_FOLDER_OF_WHOLE_OUTPUTS_IS_OPEN_TO_OTHER_USERS: &str =
    "the folder of whole outputs is open to other users";
const THE_FOLDER_OF_WHOLE_OUTPUTS_BELONGS_TO_ANOTHER_USER: &str =
    "the folder of whole outputs belongs to another user";

/// Most mebibytes kept of one output: what comes after is not kept.
pub const MOST_MEBIBYTES_KEPT: usize = 64;

/// Where the outputs that were cut are kept whole, for a day.
///
/// Each one is a file only its owner can read, in a folder that is its owner's and that only
/// its owner can open: an output may hold what a command printed of a secret. No more than
/// [`MOST_MEBIBYTES_KEPT`] mebibytes are kept of an output. When the folder is refused, the
/// file goes to the fallback folder, if there is one, under the same checks.
pub struct WholeOutputs {
    folder: PathBuf,
    fallback: Option<PathBuf>,
    name: String,
    now: SystemTime,
    pub(crate) most_kept: usize,
}

impl WholeOutputs {
    /// Creates the keeper of the output of one command, run now by the process `process`: its
    /// file in `folder` is named after the second of the day and the process, so that two
    /// commands of the same day never share one, in as few characters as that takes.
    #[must_use]
    pub fn new(folder: PathBuf, now: SystemTime, process: u32) -> Self {
        let second_of_the_day = now
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() % SECONDS_IN_A_DAY);
        Self {
            folder,
            fallback: None,
            name: format!("{second_of_the_day}-{process}"),
            now,
            most_kept: MOST_MEBIBYTES_KEPT * BYTES_IN_A_MEBIBYTE,
        }
    }

    /// Returns the same keeper, which keeps an output in `fallback` when its own folder is
    /// refused: another user made it, it is open to others, or it cannot be made.
    #[must_use]
    pub fn or_else_in(self, fallback: PathBuf) -> Self {
        Self {
            fallback: Some(fallback),
            ..self
        }
    }

    /// Writes `output` to its file and returns the path, after forgetting the outputs kept for
    /// more than a day.
    ///
    /// # Errors
    ///
    /// Returns an error if, in the folder and then in the fallback, the folder cannot be
    /// created, is open to other users or belongs to one, or the file already exists; or if the
    /// file cannot be written.
    pub fn keep(&self, output: &[u8]) -> io::Result<PathBuf> {
        let mut kept = self.start()?;
        kept.write(output)?;
        Ok(kept.path)
    }

    /// Creates the file, empty, for an output that is kept as it comes, after forgetting the
    /// outputs kept for more than a day.
    ///
    /// # Errors
    ///
    /// Returns an error if, in the folder and then in the fallback, the folder cannot be
    /// created, is open to other users or belongs to one, or the file already exists.
    pub fn start(&self) -> io::Result<Kept> {
        self.start_in(&self.folder).or_else(|refusal| {
            self.fallback
                .as_deref()
                .map_or(Err(refusal), |fallback| self.start_in(fallback))
        })
    }

    fn start_in(&self, folder_path: &Path) -> io::Result<Kept> {
        DirBuilder::new()
            .recursive(true)
            .mode(FOLDER_OF_ITS_OWNER)
            .create(folder_path)?;
        let folder = fs::symlink_metadata(folder_path)?;
        if !folder.is_dir() || folder.mode() & OPEN_TO_OTHERS != 0 {
            return Err(refused(THE_FOLDER_OF_WHOLE_OUTPUTS_IS_OPEN_TO_OTHER_USERS));
        }
        self.forget_the_old(folder_path);

        let path = folder_path.join(&self.name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_OF_ITS_OWNER)
            .open(&path)?;
        if let Err(refusal) = of_one_owner(folder.uid(), file.metadata()?.uid()) {
            fs::remove_file(&path)?;
            return Err(refusal);
        }
        Ok(Kept {
            file,
            path,
            room: self.most_kept,
            is_whole: true,
        })
    }

    fn forget_the_old(&self, folder: &Path) {
        let Ok(kept) = fs::read_dir(folder) else {
            return;
        };
        for file in kept.flatten() {
            let written = file.metadata().and_then(|metadata| metadata.modified());
            let is_old = written.is_ok_and(|written| {
                self.now
                    .duration_since(written)
                    .is_ok_and(|age| age > KEPT_FOR)
            });
            if is_old {
                fs::remove_file(file.path()).unwrap_or_default();
            }
        }
    }
}

/// Returns where outputs are kept whole inside the user's own temporary folder, `named` by the
/// environment: `None` when it is not named, or not by an absolute path.
#[must_use]
pub fn in_own_temporary_folder(named: Option<OsString>) -> Option<PathBuf> {
    named
        .map(PathBuf::from)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(WHOLE_OUTPUTS_FOLDER))
}

fn of_one_owner(folder_owner: u32, file_owner: u32) -> io::Result<()> {
    if folder_owner == file_owner {
        return Ok(());
    }
    Err(refused(THE_FOLDER_OF_WHOLE_OUTPUTS_BELONGS_TO_ANOTHER_USER))
}

fn refused(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, reason)
}

/// The file of an output that is being kept whole.
pub struct Kept {
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    pub(crate) room: usize,
    pub(crate) is_whole: bool,
}

impl Kept {
    /// Writes the next `piece` of the output at the end of the file, as far as the file may
    /// still grow.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write(&mut self, piece: &[u8]) -> io::Result<()> {
        let fitting = &piece[..piece.len().min(self.room)];
        self.room -= fitting.len();
        self.is_whole &= fitting.len() == piece.len();
        self.file.write_all(fitting)
    }

    /// Returns whether the file holds all of the output so far: `false` once the output went
    /// past what is kept of one.
    #[must_use]
    pub const fn is_whole(&self) -> bool {
        self.is_whole
    }

    /// Returns where the output is kept.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Removes the file of an output nobody needs whole.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be removed.
    pub fn forget(self) -> io::Result<()> {
        fs::remove_file(self.path)
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-shell/whole.rs"]
mod tests;
