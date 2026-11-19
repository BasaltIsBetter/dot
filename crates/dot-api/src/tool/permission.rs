use crate::cancellation::Cancellation;
use crate::text::Text;
use crate::tool::policy::is_protected;

const SCHEME_END: &str = "://";
const AUTHORITY_ENDS: [char; 3] = ['/', '?', '#'];
const USER_INFO_END: char = '@';
const PORT_START: char = ':';
const BRACKETED_HOST_END: char = ']';

/// Something Claude wants to do that needs the user's yes first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permission {
    /// Identifier of the tool call that asks, as in [`ReplyEvent::ToolStarted`](crate::ReplyEvent).
    pub tool_id: String,
    /// What the call would do.
    pub action: Action,
}

/// What a tool call would do, as the user is asked about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run a shell command.
    Run {
        /// Command line.
        command: String,
    },
    /// Create or change a file inside the session's folder.
    Change {
        /// File to write.
        path: String,
    },
    /// Create or change a file outside the session's folder.
    ChangeOutside {
        /// File to write.
        path: String,
    },
    /// Read a file or list a folder outside the session's folder.
    ReadOutside {
        /// File or folder to read.
        path: String,
    },
    /// Fetch a web page.
    Fetch {
        /// Address of the page.
        url: String,
    },
    /// Search the web.
    SearchWeb {
        /// What to search for.
        query: String,
    },
    /// Use another tool.
    Use {
        /// Name of the tool.
        tool: String,
        /// What it would work on, when known.
        detail: String,
    },
}

/// The user's answer to a [`Permission`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Claude may go ahead.
    Allow,
    /// Claude may not; it reads that the user refused.
    Deny,
}

/// How Dot answers what Claude asks to do on the Mac, by either connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionMode {
    /// Puts each action that needs it to the user, then waits for their answer.
    Ask,
    /// Allows every action at once, without asking the user; a stopped reply still goes no
    /// further.
    Bypass,
}

/// What a "yes, always" answer covers until the window closes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Grant {
    /// This exact command line.
    Command(String),
    /// Every file created or changed inside the session's folder, except protected ones.
    Changes,
    /// This file inside the session's folder, protected because changing it can make code run
    /// later (see [`Policy`](crate::Policy)).
    ProtectedFile(String),
    /// This file, outside the session's folder, to change.
    ChangeOutside(String),
    /// This file or folder, outside the session's folder, to read.
    ReadOutside(String),
    /// Every page of this host.
    Host(String),
    /// Every web search.
    SearchWeb,
    /// Every use of this tool.
    Tool(String),
}

/// Asks the user whether Claude may go ahead with what it wants to do.
pub trait Permissions {
    /// Asks the user about `permission` and blocks until they answer.
    ///
    /// Raising `cancellation` ends the wait at once with [`Decision::Deny`]: a stopped reply
    /// never goes ahead.
    fn ask(&self, permission: &Permission, cancellation: &Cancellation) -> Decision;
}

impl Action {
    /// Returns what a "yes, always" answer to this action covers.
    ///
    /// A command covers itself only; a change inside the folder covers every change inside it,
    /// except a protected file, which covers itself only; a page covers its host.
    #[must_use]
    pub fn grant(&self) -> Grant {
        match self {
            Self::Run { command } => Grant::Command(command.clone()),
            Self::Change { path } if is_protected(path) => Grant::ProtectedFile(path.clone()),
            Self::Change { .. } => Grant::Changes,
            Self::ChangeOutside { path } => Grant::ChangeOutside(path.clone()),
            Self::ReadOutside { path } => Grant::ReadOutside(path.clone()),
            Self::Fetch { url } => Grant::Host(host(url)),
            Self::SearchWeb { .. } => Grant::SearchWeb,
            Self::Use { tool, .. } => Grant::Tool(tool.clone()),
        }
    }

    /// Returns the text that asks the user, shown in front of what the action works on.
    #[must_use]
    pub const fn text(&self) -> Text {
        match self {
            Self::Run { .. } => Text::PermissionRun,
            Self::Change { .. } => Text::PermissionChange,
            Self::ChangeOutside { .. } => Text::PermissionChangeOutside,
            Self::ReadOutside { .. } => Text::PermissionReadOutside,
            Self::Fetch { .. } => Text::PermissionFetch,
            Self::SearchWeb { .. } => Text::PermissionSearchWeb,
            Self::Use { .. } => Text::PermissionUse,
        }
    }

    /// Returns what the action works on, as the user reads it.
    #[must_use]
    pub fn subject(&self) -> &str {
        match self {
            Self::Run { command } => command,
            Self::Change { path } | Self::ChangeOutside { path } | Self::ReadOutside { path } => {
                path
            }
            Self::Fetch { url } => url,
            Self::SearchWeb { query } => query,
            Self::Use { tool, .. } => tool,
        }
    }
}

impl Grant {
    /// Returns the text that tells the user what "yes, always" covers.
    #[must_use]
    pub const fn text(&self) -> Text {
        match self {
            Self::Command(_) => Text::GrantCommand,
            Self::Changes => Text::GrantChanges,
            Self::ProtectedFile(_) | Self::ChangeOutside(_) | Self::ReadOutside(_) => {
                Text::GrantFile
            }
            Self::Host(_) => Text::GrantHost,
            Self::SearchWeb => Text::GrantSearchWeb,
            Self::Tool(_) => Text::GrantTool,
        }
    }
}

fn host(url: &str) -> String {
    let after_scheme = url.split_once(SCHEME_END).map_or(url, |(_, rest)| rest);
    let authority = after_scheme
        .split(AUTHORITY_ENDS)
        .next()
        .unwrap_or_default();
    let host_and_port = authority
        .rsplit_once(USER_INFO_END)
        .map_or(authority, |(_, host)| host);
    let host = match host_and_port.split_once(BRACKETED_HOST_END) {
        Some((bracketed, _)) => &host_and_port[..=bracketed.len()],
        None => host_and_port.split(PORT_START).next().unwrap_or_default(),
    };
    host.to_ascii_lowercase()
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/tool/permission.rs"]
mod tests;
