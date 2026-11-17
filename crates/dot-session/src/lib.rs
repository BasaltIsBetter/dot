//! Dot sessions: the work of one window with Claude, in one folder.
//!
//! The folder comes from the launch ([`launch_folder`]). A [`LocalSession`] reports the folder's
//! name and its git branch, read without leaving the folder, along with what the session has
//! consumed.

mod git;
mod launch;
mod local;
mod readable;

pub use crate::launch::launch_folder;
pub use crate::local::LocalSession;
