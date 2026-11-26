use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing::Level;

const LOG_FILE: &str = "dot.log";
const PREVIOUS_LOG_FILE: &str = "dot.log.1";
const LONGEST_LOG: u64 = 1_024 * 1_024;

pub(crate) struct Log {
    folder: PathBuf,
    file: File,
    length: u64,
}

impl Log {
    pub(crate) fn open(folder: &Path) -> io::Result<Self> {
        fs::create_dir_all(folder)?;
        let path = folder.join(LOG_FILE);
        let full = fs::metadata(&path).is_ok_and(|metadata| metadata.len() > LONGEST_LOG);
        if full {
            fs::rename(&path, folder.join(PREVIOUS_LOG_FILE))?;
        }
        let file = append_to(&path)?;
        let length = file.metadata()?.len();
        Ok(Self {
            folder: folder.to_path_buf(),
            file,
            length,
        })
    }

    fn would_pass_the_limit(&self, more: usize) -> bool {
        let more = u64::try_from(more).unwrap_or(u64::MAX);
        self.length > 0 && self.length.saturating_add(more) > LONGEST_LOG
    }

    fn start_afresh(&mut self) -> io::Result<()> {
        let path = self.folder.join(LOG_FILE);
        fs::rename(&path, self.folder.join(PREVIOUS_LOG_FILE))?;
        self.file = append_to(&path)?;
        self.length = 0;
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file.write_all(bytes)?;
        self.length = self
            .length
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        Ok(())
    }
}

impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.would_pass_the_limit(bytes.len())
            && let Err(error) = self.start_afresh()
        {
            self.length = 0;
            self.append(format!("the log cannot start afresh: {error}\n").as_bytes())?;
        }
        self.append(bytes)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub(crate) fn start_logging(log: Log) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(Level::INFO)
        .with_writer(Mutex::new(log))
        .try_init()
}

fn append_to(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
#[path = "../../../tests/dot/log.rs"]
mod tests;
