mod stored;

use std::path::PathBuf;

use dot_api::{Preferences, PreferencesError, Settings};

use crate::private::{read_at_most, write_whole};

pub(crate) use self::stored::{hue_name, hue_named};

const FILE_NAME: &str = "settings.json";
const LARGEST_FILE: u64 = 64 * 1_024;
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
        read_at_most(&self.folder.as_ref()?.join(FILE_NAME), LARGEST_FILE)
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
        write_whole(folder, FILE_NAME, |file| {
            file.write_all(&stored::written(settings, defaults))
        })
        .map_err(|error| PreferencesError(error.to_string()))
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/settings/mod.rs"]
mod tests;
