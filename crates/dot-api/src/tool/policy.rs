use std::path::{Component, Path};

use crate::tool::permission::{Action, Grant, PermissionMode};

const PROTECTED_FOLDERS: [&str; 4] = [".git", ".claude", ".cargo", ".vscode"];
const PROTECTED_FILES: [&str; 1] = [".envrc"];

/// The one place that decides what Claude may do without asking, for one window.
///
/// Built from the [`PermissionMode`] of the user's settings, it gathers the "yes, always"
/// answers the user gives along the way. Changes to what makes code run later in the folder (git hooks and
/// settings, Claude Code, Cargo and editor settings, `.envrc`) are protected: a "yes, always" on
/// the folder's changes does not cover them, each asks for itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    mode: PermissionMode,
    grants: Vec<Grant>,
}

/// What a [`Policy`] says of an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Claude may go ahead without asking.
    Allow,
    /// The user must be asked first.
    Ask,
}

impl Policy {
    /// Creates the policy of a window, before the user has granted anything.
    #[must_use]
    pub const fn new(mode: PermissionMode) -> Self {
        Self {
            mode,
            grants: Vec::new(),
        }
    }

    /// Returns whether `action` may go ahead without asking the user.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::{Action, PermissionMode, Policy, Verdict};
    ///
    /// let mut policy = Policy::new(PermissionMode::Ask);
    /// let change = Action::Change { path: "src/main.rs".to_owned() };
    /// let hook = Action::Change { path: ".git/hooks/pre-commit".to_owned() };
    ///
    /// policy.grant_always(&change);
    /// assert_eq!(policy.judge(&change), Verdict::Allow);
    /// assert_eq!(policy.judge(&hook), Verdict::Ask);
    /// ```
    #[must_use]
    pub fn judge(&self, action: &Action) -> Verdict {
        if self.mode == PermissionMode::Bypass || self.grants.contains(&action.grant()) {
            return Verdict::Allow;
        }
        Verdict::Ask
    }

    /// Judges the next actions by `mode`, chosen again in the settings. The "yes, always"
    /// answers given so far are kept.
    pub const fn switch_to(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }

    /// Lets every action covered like `action` go ahead without asking, until the window closes.
    pub fn grant_always(&mut self, action: &Action) {
        let grant = action.grant();
        if !self.grants.contains(&grant) {
            self.grants.push(grant);
        }
    }
}

/// Returns whether changing `path`, inside the session's folder, can make code run later:
/// anything in a `.git`, `.claude`, `.cargo` or `.vscode` folder, or a `.envrc` file, whatever
/// the case of their names: the disks of macOS do not tell `.GIT` from `.git`.
#[must_use]
pub fn is_protected(path: &str) -> bool {
    let path = Path::new(path);
    let in_protected_folder = path.components().any(|component| match component {
        Component::Normal(name) => PROTECTED_FOLDERS
            .iter()
            .any(|folder| name.eq_ignore_ascii_case(folder)),
        _ => false,
    });
    let protected_file = path.file_name().is_some_and(|name| {
        PROTECTED_FILES
            .iter()
            .any(|file| name.eq_ignore_ascii_case(file))
    });
    in_protected_folder || protected_file
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/tool/policy.rs"]
mod tests;
