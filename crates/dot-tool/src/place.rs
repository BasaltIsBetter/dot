use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const CURRENT_FOLDER: &str = ".";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Place {
    pub(crate) path: PathBuf,
    pub(crate) inside: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlaceError {
    Empty,
    Nul,
    NoFolder,
    UpPastExisting,
    Unresolvable(String),
}

impl fmt::Display for PlaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("The path is empty."),
            Self::Nul => formatter.write_str("The path contains a NUL character."),
            Self::NoFolder => {
                formatter.write_str("The session has no folder: give an absolute path.")
            }
            Self::UpPastExisting => formatter.write_str(
                "The path goes up with `..` below a folder that does not exist: write it without `..`.",
            ),
            Self::Unresolvable(path) => write!(
                formatter,
                "{path} exists but cannot be followed: a broken link, a loop of links, or no access."
            ),
        }
    }
}

pub(crate) fn locate(folder: Option<&Path>, raw: &str) -> Result<Place, PlaceError> {
    let path = resolve(folder, raw)?;
    let inside = folder.is_some_and(|folder| path.starts_with(folder));
    Ok(Place { path, inside })
}

pub(crate) fn resolve(folder: Option<&Path>, raw: &str) -> Result<PathBuf, PlaceError> {
    if raw.is_empty() {
        return Err(PlaceError::Empty);
    }
    if raw.contains('\0') {
        return Err(PlaceError::Nul);
    }
    let asked = Path::new(raw);
    let joined = if asked.is_absolute() {
        asked.to_path_buf()
    } else {
        folder.ok_or(PlaceError::NoFolder)?.join(asked)
    };

    for existing in joined.ancestors() {
        match fs::canonicalize(existing) {
            Ok(base) => return append_missing(base, &joined, existing),
            Err(_) if fs::symlink_metadata(existing).is_ok() => {
                return Err(PlaceError::Unresolvable(existing.display().to_string()));
            }
            Err(_) => {}
        }
    }
    Err(PlaceError::Unresolvable(joined.display().to_string()))
}

pub(crate) fn shown(place: &Place, folder: Option<&Path>) -> String {
    let relative = folder
        .filter(|_| place.inside)
        .and_then(|folder| place.path.strip_prefix(folder).ok());
    match relative {
        Some(relative) if relative.as_os_str().is_empty() => CURRENT_FOLDER.to_owned(),
        Some(relative) => relative.display().to_string(),
        None => place.path.display().to_string(),
    }
}

fn append_missing(base: PathBuf, joined: &Path, existing: &Path) -> Result<PathBuf, PlaceError> {
    let missing = joined.strip_prefix(existing).unwrap_or(Path::new(""));
    let mut path = base;
    for component in missing.components() {
        match component {
            Component::Normal(name) => path.push(name),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PlaceError::UpPastExisting);
            }
        }
    }
    Ok(path)
}

#[cfg(test)]
#[path = "../../../tests/dot-tool/place.rs"]
mod tests;
