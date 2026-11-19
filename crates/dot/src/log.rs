use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing::Level;

const LOG_FOLDER: &str = "Library/Logs/Dot";
const LOG_FILE: &str = "dot.log";
const PREVIOUS_LOG_FILE: &str = "dot.log.1";
const LONGEST_LOG: u64 = 1_024 * 1_024;

pub(crate) fn log_folder(home: &Path) -> PathBuf {
    home.join(LOG_FOLDER)
}

pub(crate) fn open_log(folder: &Path) -> io::Result<File> {
    fs::create_dir_all(folder)?;
    let path = folder.join(LOG_FILE);
    let full = fs::metadata(&path).is_ok_and(|metadata| metadata.len() > LONGEST_LOG);
    if full {
        fs::rename(&path, folder.join(PREVIOUS_LOG_FILE))?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

pub(crate) fn start_logging(file: File) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(Level::INFO)
        .with_writer(Mutex::new(file))
        .try_init()
}

#[cfg(test)]
#[path = "../../../tests/dot/log.rs"]
mod tests;
