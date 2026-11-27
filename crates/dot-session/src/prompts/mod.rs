use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use dot_api::{ArchiveError, PromptHistory};
use serde::Serialize;
use serde_json::Value;

use crate::private::{read_at_most, write_whole};

const FILE_NAME: &str = "prompts.json";
const LARGEST_FILE: u64 = 4 * 1_024 * 1_024;
const PROMPTS_FORMAT: u64 = 1;
const FORMAT: &str = "format";
const PROMPTS: &str = "prompts";
const NO_FOLDER: &str = "Dot has no folder to keep the prompts in";

/// [`PromptHistory`] kept in `prompts.json`, in Dot's own folder of this machine.
///
/// The file is written whole beside the one it replaces, then put in its place, readable by its
/// owner only; a file larger than 4 MiB, or of another format, is read as no prompt at all.
#[derive(Debug)]
pub struct FilePromptHistory {
    folder: Option<PathBuf>,
    saving: Mutex<()>,
}

#[derive(Serialize)]
struct Written<'a> {
    format: u64,
    prompts: &'a [String],
}

impl FilePromptHistory {
    /// Keeps the prompts in `folder`, created when they are first saved; without a folder,
    /// nothing is kept.
    #[must_use]
    pub const fn new(folder: Option<PathBuf>) -> Self {
        Self {
            folder,
            saving: Mutex::new(()),
        }
    }
}

impl PromptHistory for FilePromptHistory {
    fn load(&self) -> Vec<String> {
        let Some(folder) = &self.folder else {
            return Vec::new();
        };
        let Some(written) = read_at_most(&folder.join(FILE_NAME), LARGEST_FILE + 1) else {
            return Vec::new();
        };
        if u64::try_from(written.len()).is_ok_and(|length| length > LARGEST_FILE) {
            return Vec::new();
        }
        let Ok(kept) = serde_json::from_slice::<Value>(&written) else {
            return Vec::new();
        };
        if kept.get(FORMAT).and_then(Value::as_u64) != Some(PROMPTS_FORMAT) {
            return Vec::new();
        }
        kept.get(PROMPTS)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|prompt| !prompt.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn save(&self, prompts: &[String]) -> Result<(), ArchiveError> {
        let folder = self
            .folder
            .as_deref()
            .ok_or_else(|| ArchiveError(NO_FOLDER.to_owned()))?;
        let written = serde_json::to_vec(&Written {
            format: PROMPTS_FORMAT,
            prompts,
        })
        .map_err(|error| ArchiveError(error.to_string()))?;
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        write_whole(folder, FILE_NAME, |file| file.write_all(&written))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/prompts/mod.rs"]
mod tests;
