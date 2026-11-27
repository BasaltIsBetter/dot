use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use dot_process::open_without_waiting;

pub(crate) const PRIVATE_FILE: u32 = 0o600;
pub(crate) const PRIVATE_FOLDER: u32 = 0o700;
pub(crate) const BEING_WRITTEN: &str = ".new";

pub(crate) fn read_at_most(path: &Path, largest: u64) -> Option<Vec<u8>> {
    let file = open_ordinary(path)?;
    let mut read = Vec::new();
    file.take(largest).read_to_end(&mut read).ok()?;
    Some(read)
}

pub(crate) fn open_ordinary(path: &Path) -> Option<File> {
    let (file, metadata) = open_without_waiting(path).ok()?;
    metadata.is_file().then_some(file)
}

pub(crate) fn create_private_folder(folder: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_FOLDER)
        .create(folder)
}

pub(crate) fn open_private_file(path: &Path, options: &mut OpenOptions) -> io::Result<File> {
    options.mode(PRIVATE_FILE).open(path)
}

pub(crate) fn write_whole(
    folder: &Path,
    name: &str,
    write: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    create_private_folder(folder)?;
    let written = folder.join(format!("{name}{BEING_WRITTEN}"));
    let file = open_private_file(
        &written,
        OpenOptions::new().write(true).create(true).truncate(true),
    )?;
    let mut buffered = BufWriter::new(file);
    write(&mut buffered)?;
    let file = buffered
        .into_inner()
        .map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    fs::rename(&written, folder.join(name))
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/private/mod.rs"]
mod tests;
