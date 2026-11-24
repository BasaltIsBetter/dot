mod stored;

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::PathBuf;

use dot_api::{Preferences, PreferencesError, Settings};
use dot_process::open_without_waiting;

const FILE_NAME: &str = "settings.json";
const WRITTEN_NAME: &str = "settings.json.new";
const LARGEST_FILE: u64 = 64 * 1_024;
const PRIVATE_FILE: u32 = 0o600;
const PRIVATE_FOLDER: u32 = 0o700;
const NO_FOLDER: &str = "Dot has no folder to keep its settings in";

/// [`Preferences`] kept in `settings.json`, in Dot's own folder of this Mac.
///
/// The file holds only the settings that differ from Dot's defaults, as JSON. A setting that is
/// missing, unknown or unreadable falls back to its default, the others are kept: a damaged
/// file never costs more than the settings it damaged.
#[derive(Clone, Debug)]
pub struct FilePreferences {
    folder: Option<PathBuf>,
}

impl FilePreferences {
    /// Keeps the settings in `folder`, created when they are first saved; without a folder,
    /// nothing is kept and every launch starts from the defaults.
    #[must_use]
    pub const fn new(folder: Option<PathBuf>) -> Self {
        Self { folder }
    }

    fn read(&self) -> Option<Vec<u8>> {
        let path = self.folder.as_ref()?.join(FILE_NAME);
        let (file, metadata) = open_without_waiting(&path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        let mut written = Vec::new();
        file.take(LARGEST_FILE).read_to_end(&mut written).ok()?;
        Some(written)
    }
}

impl Preferences for FilePreferences {
    fn load(&self, defaults: &Settings) -> Settings {
        self.read()
            .map_or(*defaults, |written| stored::read(&written, defaults))
    }

    fn save(&self, settings: &Settings, defaults: &Settings) -> Result<(), PreferencesError> {
        let folder = self
            .folder
            .as_ref()
            .ok_or_else(|| PreferencesError(NO_FOLDER.to_owned()))?;
        let refused = |error: std::io::Error| PreferencesError(error.to_string());

        DirBuilder::new()
            .recursive(true)
            .mode(PRIVATE_FOLDER)
            .create(folder)
            .map_err(refused)?;
        let written = folder.join(WRITTEN_NAME);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(PRIVATE_FILE)
            .open(&written)
            .map_err(refused)?;
        file.write_all(&stored::written(settings, defaults))
            .and_then(|()| file.sync_all())
            .map_err(refused)?;
        fs::rename(&written, folder.join(FILE_NAME)).map_err(refused)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/settings/mod.rs"]
mod tests;
