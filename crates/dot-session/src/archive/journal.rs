use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dot_api::{ArchiveError, Happened, Journal, Record};
use serde_json::Value;

use crate::archive::claim::Claim;
use crate::private::{create_private_folder, open_private_file, write_whole};

pub(crate) const JOURNAL: &str = "journal";
pub(crate) const MEMORY: &str = "memory";
pub(crate) const HEADER: &[u8] = br#"{"journal":1}"#;
const FORMAT: &str = "journal";
const JOURNAL_FORMAT: u64 = 1;
const NEW_LINE: u8 = b'\n';
const NOT_A_FILE: &str = "the journal is not an ordinary file";

/// [`Journal`] of one session, in its own folder among the sessions Dot keeps.
///
/// The records go to `journal`, one per line after a line that names the format: each is the
/// time in milliseconds on the session's clock and what happened, in JSON. Nothing touches the
/// disk until the first record; the folder is then created for its owner only, the file opened
/// to append, readable by its owner only. What the session's chat remembers goes to `memory`,
/// written whole beside it then put in place of the one before.
#[derive(Debug)]
pub struct FileJournal {
    folder: Option<PathBuf>,
    claim: Arc<Claim>,
    file: Option<BufWriter<File>>,
    line: Vec<u8>,
}

impl FileJournal {
    pub(crate) const fn new(folder: Option<PathBuf>, claim: Arc<Claim>) -> Self {
        Self {
            folder,
            claim,
            file: None,
            line: Vec::new(),
        }
    }

    fn folder(&self) -> Result<&Path, ArchiveError> {
        let folder = self.folder.as_deref().ok_or_else(Claim::no_folder)?;
        self.claim.ours()?;
        Ok(folder)
    }

    fn open(&self) -> Result<BufWriter<File>, ArchiveError> {
        let folder = self.folder()?;
        create_private_folder(folder)?;
        let path = folder.join(JOURNAL);
        if fs::symlink_metadata(&path).is_ok_and(|metadata| !metadata.is_file()) {
            return Err(ArchiveError(NOT_A_FILE.to_owned()));
        }
        let mut file = open_private_file(
            &path,
            OpenOptions::new().read(true).append(true).create(true),
        )?;
        start_on_a_line(&mut file)?;
        Ok(BufWriter::new(file))
    }
}

impl Journal for FileJournal {
    fn write(&mut self, record: &Record<'_>) -> Result<(), ArchiveError> {
        if self.file.is_none() {
            self.file = Some(self.open()?);
        }
        line_of(record, &mut self.line)?;
        let Some(file) = self.file.as_mut() else {
            return Ok(());
        };
        file.write_all(&self.line)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ArchiveError> {
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
        }
        Ok(())
    }

    fn keep_memory(
        &mut self,
        chat: &dyn Fn(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), ArchiveError> {
        let folder = self.folder()?;
        write_whole(folder, MEMORY, chat)?;
        Ok(())
    }
}

pub(crate) fn line_of(record: &Record<'_>, line: &mut Vec<u8>) -> Result<(), ArchiveError> {
    line.clear();
    serde_json::to_writer(&mut *line, &(milliseconds(record.at), &record.happened))
        .map_err(|error| ArchiveError(error.to_string()))?;
    line.push(NEW_LINE);
    Ok(())
}

pub(crate) fn record_of(line: &[u8]) -> Option<Record<'static>> {
    let (at, happened) = serde_json::from_slice::<(u64, Happened<'static>)>(line).ok()?;
    Some(Record {
        at: Duration::from_millis(at),
        happened,
    })
}

pub(crate) fn is_header(line: &[u8]) -> bool {
    serde_json::from_slice::<Value>(line)
        .is_ok_and(|header| header.get(FORMAT).and_then(Value::as_u64) == Some(JOURNAL_FORMAT))
}

fn milliseconds(at: Duration) -> u64 {
    u64::try_from(at.as_millis()).unwrap_or(u64::MAX)
}

fn start_on_a_line(file: &mut File) -> io::Result<()> {
    if file.metadata()?.len() == 0 {
        file.write_all(HEADER)?;
        return file.write_all(&[NEW_LINE]);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0];
    file.read_exact(&mut last)?;
    if last[0] == NEW_LINE {
        return Ok(());
    }
    file.write_all(&[NEW_LINE])
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/archive/journal.rs"]
pub(crate) mod tests;
