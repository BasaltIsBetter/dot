#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::{places, preferred_languages};
#[cfg(target_os = "macos")]
pub(crate) use self::macos::{places, preferred_languages};

/// Where Dot keeps its files on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Places {
    pub(crate) workspace: PathBuf,
    pub(crate) logs: PathBuf,
}
