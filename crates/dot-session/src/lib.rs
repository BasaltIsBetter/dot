//! Dot sessions: the work of one window with Claude, in one folder.
//!
//! The folder comes from the launch ([`launch_folder`]). A [`LocalSession`] reports the folder's
//! name and its git branch, read without leaving the folder, along with what the session has
//! consumed. The user's settings, the same for every session, are kept between launches by
//! [`FilePreferences`]; the sessions of the window, their conversations and what their chats
//! remember by [`FileArchive`], each session's in its [`FileJournal`]; the prompts the user sent
//! by [`FilePromptHistory`].

mod archive;
mod git;
mod launch;
mod local;
mod private;
mod prompts;
mod readable;
mod settings;

pub use crate::archive::{FileArchive, FileJournal};
pub use crate::launch::launch_folder;
pub use crate::local::LocalSession;
pub use crate::prompts::FilePromptHistory;
pub use crate::settings::FilePreferences;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
