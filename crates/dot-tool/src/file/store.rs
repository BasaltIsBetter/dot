use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process;

use dot_process::open_without_waiting;

use crate::text::NEW_LINE;

pub(crate) const LARGEST_EDITED: u64 = 64 * 1_024 * 1_024;
const TEMPORARY_ATTEMPTS: u32 = 64;
const OWNER_ALONE: u32 = 0o600;
const LONGEST_NAME: usize = 255;
const TEMPORARY_MARK: char = '.';

pub(crate) struct Original {
    pub(crate) bytes: Vec<u8>,
    pub(crate) permissions: Option<Permissions>,
}

pub(crate) fn not_a_file(path: &Path) -> String {
    format!(
        "{} is not a regular file (a pipe, a socket or a device): Claude cannot read it.",
        path.display()
    )
}

pub(crate) fn read_original(path: &Path) -> Result<Option<Original>, String> {
    open_if_there(path)?
        .map(|opened| read_bounded(path, opened))
        .transpose()
}

pub(crate) fn read_text(path: &Path) -> Result<(String, Original), String> {
    let opened = open_without_waiting(path).map_err(|error| failure(path, &error))?;
    let mut original = read_bounded(path, opened)?;
    let bytes = std::mem::take(&mut original.bytes);
    let text = String::from_utf8(bytes).map_err(|_| {
        format!(
            "{} is not UTF-8 text: Claude cannot edit it.",
            path.display()
        )
    })?;
    Ok((text, original))
}

pub(crate) fn read_first_line(path: &Path) -> Result<Option<Original>, String> {
    let Some((file, metadata)) = open_if_there(path)? else {
        return Ok(None);
    };
    a_file(path, &metadata)?;
    let mut bytes = Vec::new();
    BufReader::new(file.take(LARGEST_EDITED + 1))
        .read_until(NEW_LINE, &mut bytes)
        .map_err(|error| failure(path, &error))?;
    Ok(Some(Original {
        bytes,
        permissions: Some(metadata.permissions()),
    }))
}

fn open_if_there(path: &Path) -> Result<Option<(File, Metadata)>, String> {
    match open_without_waiting(path) {
        Ok(opened) => Ok(Some(opened)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failure(path, &error)),
    }
}

fn a_file(path: &Path, metadata: &Metadata) -> Result<(), String> {
    if metadata.is_dir() {
        return Err(format!("{} is a folder, not a file.", path.display()));
    }
    if !metadata.is_file() {
        return Err(not_a_file(path));
    }
    Ok(())
}

fn read_bounded(path: &Path, (file, metadata): (File, Metadata)) -> Result<Original, String> {
    a_file(path, &metadata)?;
    if metadata.len() > LARGEST_EDITED {
        return Err(format!(
            "{} is larger than 64 MiB: too large to edit this way.",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(LARGEST_EDITED + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| failure(path, &error))?;
    Ok(Original {
        bytes,
        permissions: Some(metadata.permissions()),
    })
}

pub(crate) fn write_atomically(
    path: &Path,
    bytes: &[u8],
    permissions: Option<Permissions>,
) -> io::Result<()> {
    let (temporary, mut file) = temporary_beside(path, permissions.is_some())?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| permissions.map_or(Ok(()), |permissions| file.set_permissions(permissions)))
        .and_then(|()| fs::rename(&temporary, path));
    if written.is_err() {
        let _cleanup = fs::remove_file(&temporary);
    }
    written
}

fn temporary_beside(path: &Path, replaces_a_file: bool) -> io::Result<(PathBuf, File)> {
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let folder = path.parent().unwrap_or(Path::new("."));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    if replaces_a_file {
        options.mode(OWNER_ALONE);
    }
    let mut last_error = io::Error::other("no attempt");
    for attempt in 0..TEMPORARY_ATTEMPTS {
        let temporary = folder.join(temporary_name(&name, attempt));
        match options.open(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn temporary_name(name: &str, attempt: u32) -> String {
    let ending = format!(".dot-{}-{attempt}.tmp", process::id());
    let room = LONGEST_NAME.saturating_sub(ending.len() + TEMPORARY_MARK.len_utf8());
    let kept = &name[..name.floor_char_boundary(room)];
    format!("{TEMPORARY_MARK}{kept}{ending}")
}

pub(crate) fn failure(path: &Path, error: &io::Error) -> String {
    format!("{}: {error}.", path.display())
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/store.rs"]
mod tests;
