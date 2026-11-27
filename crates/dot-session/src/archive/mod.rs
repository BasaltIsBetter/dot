mod claim;
mod index;
mod journal;
mod shelf;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use dot_api::{Archive, ArchiveError, Arrangement, KeptId, Restored, Shelf};

use crate::private::{read_at_most, write_whole};

use self::claim::Claim;
use self::index::INDEX;

pub use self::journal::FileJournal;

const LARGEST_INDEX: u64 = 1_024 * 1_024;

/// [`Archive`] of the sessions in a folder of Dot's own, `sessions` in Dot's folder of this
/// machine.
///
/// `index.json` lists the sessions of the window in order, each with its name, its colour and
/// its folder, and the session shown. Each session keeps its [`FileJournal`] in a folder named
/// after its [`KeptId`]. Every file is readable by its owner only, and every file but the
/// journals is written whole beside the one it replaces, then put in its place: never half
/// written. A lock on a file of the folder, taken the first time it is needed, keeps a second Dot
/// from writing the same sessions: that Dot keeps none.
#[derive(Debug)]
pub struct FileArchive {
    folder: Option<PathBuf>,
    claim: Arc<Claim>,
    arranging: Mutex<()>,
}

impl FileArchive {
    /// Keeps the sessions in `folder`, created when something is first kept; without a folder,
    /// nothing is kept and every launch starts with a new session.
    #[must_use]
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            claim: Arc::new(Claim::new(folder.clone())),
            folder,
            arranging: Mutex::new(()),
        }
    }

    fn arrangement(folder: &Path) -> Option<Arrangement> {
        read_at_most(&folder.join(INDEX), LARGEST_INDEX).and_then(|written| index::read(&written))
    }
}

impl Archive for FileArchive {
    type Journal = FileJournal;

    fn restore(&self, alive: &[KeptId]) -> Shelf {
        let Some(folder) = &self.folder else {
            return Shelf::default();
        };
        if let Err(error) = self.claim.ours() {
            tracing::warn!(%error, "the sessions of this window will not be kept");
            return Shelf::default();
        }
        let Some(arrangement) = Self::arrangement(folder) else {
            return Shelf::default();
        };

        forget_all_but(folder, &arrangement, alive);
        let sessions = arrangement
            .sessions
            .into_iter()
            .map(|session| {
                let kept = folder.join(session.id.to_string());
                Restored {
                    records: shelf::records_of(&kept),
                    memory: shelf::memory_of(&kept),
                    session,
                }
            })
            .collect();
        Shelf {
            sessions,
            shown: arrangement.shown,
        }
    }

    fn journal(&self, id: KeptId) -> FileJournal {
        let folder = self
            .folder
            .as_ref()
            .map(|folder| folder.join(id.to_string()));
        FileJournal::new(folder, Arc::clone(&self.claim))
    }

    fn arrange(&self, arrangement: &Arrangement) -> Result<(), ArchiveError> {
        let folder = self.folder.as_deref().ok_or_else(Claim::no_folder)?;
        self.claim.ours()?;
        let written =
            index::written(arrangement).map_err(|error| ArchiveError(error.to_string()))?;
        let _arranging = self
            .arranging
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        write_whole(folder, INDEX, |file| file.write_all(&written))?;
        Ok(())
    }
}

fn forget_all_but(folder: &Path, arrangement: &Arrangement, alive: &[KeptId]) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(id) = name.to_str().and_then(KeptId::parse) else {
            continue;
        };
        let kept = arrangement.sessions.iter().any(|session| session.id == id);
        if kept || alive.contains(&id) {
            continue;
        }
        if let Err(error) = fs::remove_dir_all(entry.path()) {
            tracing::warn!(%error, %id, "a closed session could not be forgotten");
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/archive/mod.rs"]
pub(crate) mod tests;
