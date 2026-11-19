use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

pub(crate) const LARGEST_EDITED: u64 = 64 * 1_024 * 1_024;
const TEMPORARY_ATTEMPTS: u32 = 64;

pub(crate) struct Original {
    pub(crate) bytes: Vec<u8>,
    pub(crate) permissions: Option<Permissions>,
}

pub(crate) fn read_original(path: &Path) -> Result<Option<Original>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failure(path, &error)),
    };
    read_bounded(path, file).map(Some)
}

pub(crate) fn read_text(path: &Path) -> Result<(String, Original), String> {
    let file = File::open(path).map_err(|error| failure(path, &error))?;
    let mut original = read_bounded(path, file)?;
    let bytes = std::mem::take(&mut original.bytes);
    let text = String::from_utf8(bytes).map_err(|_| {
        format!(
            "{} is not UTF-8 text: Claude cannot edit it.",
            path.display()
        )
    })?;
    Ok((text, original))
}

fn read_bounded(path: &Path, file: File) -> Result<Original, String> {
    let metadata = file.metadata().map_err(|error| failure(path, &error))?;
    if metadata.is_dir() {
        return Err(format!("{} is a folder, not a file.", path.display()));
    }
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
    let (temporary, mut file) = temporary_beside(path)?;
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

fn temporary_beside(path: &Path) -> io::Result<(PathBuf, File)> {
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let folder = path.parent().unwrap_or(Path::new("."));
    let mut last_error = io::Error::other("no attempt");
    for attempt in 0..TEMPORARY_ATTEMPTS {
        let temporary = folder.join(format!(".{name}.dot-{}-{attempt}.tmp", process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

pub(crate) fn failure(path: &Path, error: &io::Error) -> String {
    format!("{}: {error}.", path.display())
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/store.rs"]
mod tests;
