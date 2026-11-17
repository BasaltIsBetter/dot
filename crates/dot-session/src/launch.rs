use std::ffi::OsString;
use std::path::PathBuf;

const OPTION_PREFIX: &[u8] = b"-";

/// Returns the folder a session opens in: the first launch argument that is not an option, or
/// else the current folder.
///
/// A relative argument is taken from the current folder. The root of the disk, where macOS starts
/// an app opened from the Finder or the Dock, is no project and yields `None`, as does a missing
/// current folder. Nothing is read from the disk: the folder may not exist.
///
/// # Examples
///
/// ```
/// use std::ffi::OsString;
/// use std::path::PathBuf;
///
/// let arguments = [OsString::from("-psn_0_42"), OsString::from("Dot")];
/// let folder = dot_session::launch_folder(arguments, Some(PathBuf::from("/Users/me")));
///
/// assert_eq!(folder, Some(PathBuf::from("/Users/me/Dot")));
/// ```
#[must_use]
pub fn launch_folder(
    arguments: impl IntoIterator<Item = OsString>,
    current_folder: Option<PathBuf>,
) -> Option<PathBuf> {
    let argument = arguments
        .into_iter()
        .find(|argument| !argument.as_encoded_bytes().starts_with(OPTION_PREFIX))
        .map(PathBuf::from);

    let folder = match argument {
        Some(path) if path.is_absolute() => path,
        Some(path) => current_folder?.join(path),
        None => current_folder?,
    };
    folder.parent().is_some().then_some(folder)
}

#[cfg(test)]
#[path = "../../../tests/dot-session/launch.rs"]
mod tests;
