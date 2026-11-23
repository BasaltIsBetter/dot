use std::fs;
use std::path::{Path, PathBuf};

use dot_api::{
    Action, Cancellation, Decision, Permission, Permissions, ToolCall, ToolOutput, Toolbox,
};
use dot_process::LoginPath;

use crate::file;
use crate::place::{Place, locate, shown};
use crate::search;
use crate::shell::Shell;

const NO_FOLDER_TO_SEARCH: &str =
    "The session has no folder: give the folder to search as an absolute path.";

/// [`Toolbox`] that works on this Mac, in the session's folder.
///
/// Reading inside the folder is free. Writing, running a command or reaching outside the folder
/// asks first. Paths are resolved once, links followed, before anything is asked or touched.
/// Commands run one at a time in `/bin/bash`, each in a fresh process that starts where the
/// previous one ended, and find programs in Dot's own `PATH` unless [`LocalToolbox::login_path`]
/// gives the user's. Closing the toolbox stops what those commands left running in the
/// background.
pub struct LocalToolbox {
    folder: Option<PathBuf>,
    shell: Shell,
}

struct Request<'a> {
    tool_id: &'a str,
    permissions: &'a dyn Permissions,
    cancellation: &'a Cancellation,
}

impl LocalToolbox {
    /// Creates the toolbox of a session in `folder`, or without a folder.
    ///
    /// Nothing is read or started until a tool runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use dot_api::Toolbox;
    /// use dot_tool::LocalToolbox;
    ///
    /// let toolbox = LocalToolbox::new(Some(PathBuf::from("/Users/me/project")));
    /// assert_eq!(toolbox.folder(), Some(PathBuf::from("/Users/me/project").as_path()));
    /// ```
    #[must_use]
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            shell: Shell::new(folder.clone()),
            folder,
        }
    }

    /// Has commands find programs where the user's login shell finds them, as in a terminal;
    /// in Dot's own `PATH` unless this is called.
    ///
    /// The shell runs at the first command that needs it, unless the caller has read
    /// `login_path` already.
    #[must_use]
    pub fn login_path(mut self, login_path: LoginPath) -> Self {
        self.shell = self.shell.login_path(login_path);
        self
    }

    fn root(&self) -> Option<PathBuf> {
        self.folder
            .as_deref()
            .and_then(|folder| fs::canonicalize(folder).ok())
    }

    fn place(&self, raw: &str) -> Result<Place, ToolOutput> {
        locate(self.root().as_deref(), raw).map_err(|error| ToolOutput::failed(error.to_string()))
    }

    fn search_root(
        &self,
        folder: Option<&str>,
        request: &Request<'_>,
    ) -> Result<Place, ToolOutput> {
        let place = match folder {
            Some(folder) => self.place(folder)?,
            None => self
                .place(".")
                .map_err(|_| ToolOutput::failed(NO_FOLDER_TO_SEARCH))?,
        };
        may_read(&place, request)?;
        Ok(place)
    }

    fn may_change(&self, place: &Place, request: &Request<'_>) -> Result<(), ToolOutput> {
        let action = if place.inside {
            Action::Change {
                path: shown(place, self.root().as_deref()),
            }
        } else {
            Action::ChangeOutside {
                path: place.path.display().to_string(),
            }
        };
        ask(action, request)
    }

    fn as_shown(&self, mut output: ToolOutput, place: &Place) -> ToolOutput {
        if let Some(change) = output.change.as_mut() {
            change.path = shown(place, self.root().as_deref());
        }
        output
    }

    fn dispatch(
        &mut self,
        call: &ToolCall,
        request: &Request<'_>,
        on_output: &mut dyn FnMut(&[u8]),
    ) -> Result<ToolOutput, ToolOutput> {
        let cancellation = request.cancellation;
        Ok(match call {
            ToolCall::View { path, lines } => {
                let place = self.place(path)?;
                may_read(&place, request)?;
                file::view(&place.path, *lines)
            }
            ToolCall::Create { path, text } => {
                let place = self.place(path)?;
                self.may_change(&place, request)?;
                self.as_shown(file::create(&place.path, text), &place)
            }
            ToolCall::Replace { path, old, new } => {
                let place = self.place(path)?;
                self.may_change(&place, request)?;
                self.as_shown(file::replace(&place.path, old, new), &place)
            }
            ToolCall::Insert {
                path,
                after_line,
                text,
            } => {
                let place = self.place(path)?;
                self.may_change(&place, request)?;
                self.as_shown(file::insert(&place.path, *after_line, text), &place)
            }
            ToolCall::Run { command } => {
                ask(
                    Action::Run {
                        command: command.clone(),
                    },
                    request,
                )?;
                self.shell.run(command, cancellation, on_output)
            }
            ToolCall::RestartShell => self.shell.restart(),
            ToolCall::FindFiles { pattern, folder } => {
                let root = self.search_root(folder.as_deref(), request)?;
                search::find(&root.path, pattern, cancellation)
            }
            ToolCall::SearchText(text_search) => {
                let root = self.search_root(text_search.folder.as_deref(), request)?;
                search::search(&root.path, text_search, cancellation)
            }
        })
    }
}

impl Toolbox for LocalToolbox {
    fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    fn holds(&self, path: &str) -> bool {
        locate(self.root().as_deref(), path).is_ok_and(|place| place.inside)
    }

    fn run(
        &mut self,
        call: &ToolCall,
        tool_id: &str,
        permissions: &dyn Permissions,
        cancellation: &Cancellation,
        on_output: &mut dyn FnMut(&[u8]),
    ) -> ToolOutput {
        if cancellation.is_cancelled() {
            return ToolOutput::stopped();
        }
        let request = Request {
            tool_id,
            permissions,
            cancellation,
        };
        match self.dispatch(call, &request, on_output) {
            Ok(output) | Err(output) => output,
        }
    }
}

fn may_read(place: &Place, request: &Request<'_>) -> Result<(), ToolOutput> {
    if place.inside {
        return Ok(());
    }
    ask(
        Action::ReadOutside {
            path: place.path.display().to_string(),
        },
        request,
    )
}

fn ask(action: Action, request: &Request<'_>) -> Result<(), ToolOutput> {
    let permission = Permission {
        tool_id: request.tool_id.to_owned(),
        action,
    };
    let decision = request.permissions.ask(&permission, request.cancellation);
    if request.cancellation.is_cancelled() {
        return Err(ToolOutput::stopped());
    }
    match decision {
        Decision::Allow => Ok(()),
        Decision::Deny => Err(ToolOutput::refused()),
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-tool/toolbox.rs"]
pub(crate) mod tests;
