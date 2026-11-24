//! Dot sessions: the work of one window with Claude, in one folder.
//!
//! The folder comes from the launch ([`launch_folder`]). A [`LocalSession`] reports the folder's
//! name and its git branch, read without leaving the folder, along with what the session has
//! consumed. The user's settings, the same for every session, are kept between launches by
//! [`FilePreferences`].

mod git;
mod launch;
mod local;
mod readable;
mod settings;

pub use crate::launch::launch_folder;
pub use crate::local::LocalSession;
pub use crate::settings::FilePreferences;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
