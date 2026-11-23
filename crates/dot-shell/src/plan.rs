use std::ffi::OsString;
use std::path::{Path, PathBuf};

use dot_compact::ShapeCompactor;

use crate::script::{Snapshot, asked, snapshot};

const ZSH: &str = "/bin/zsh";
const BASH: &str = "/bin/bash";
const COMMAND_FLAG: &str = "-c";
const NAME: &str = "dot-shell";

/// What the program does with what Claude Code handed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// The real shell takes the program's place, with these arguments: nothing it reads or
    /// writes is touched.
    Hand {
        /// Path of the real shell.
        shell: PathBuf,
        /// Arguments the real shell starts with.
        arguments: Vec<OsString>,
    },
    /// The real shell runs `script` and what it prints is shortened.
    Shorten {
        /// Path of the real shell.
        shell: PathBuf,
        /// Whole command line Claude Code assembled, run as it is.
        script: String,
        /// Command Claude wrote, found inside `script`.
        command: String,
    },
}

/// Returns what to do with the `arguments` Claude Code gave the program, `shell` being the
/// user's shell as the environment names it.
///
/// Only a command of Claude whose output may be shortened is planned as [`Plan::Shorten`]. A
/// hook, a status line, the start of an MCP server, and a command whose output Claude asked for
/// whole (`DOT_RAW=1`, or one that reads an output kept whole) are handed to the real shell.
/// The real shell is the one whose snapshot the command line sources, `shell` when it is one of
/// that kind, `/bin/zsh` or `/bin/bash` otherwise.
///
/// # Examples
///
/// ```
/// use std::ffi::OsString;
///
/// use dot_shell::{Plan, plan};
///
/// let hook = plan(vec![OsString::from("my-hook --check")], Some(OsString::from("/bin/zsh")));
///
/// assert_eq!(
///     hook,
///     Plan::Hand {
///         shell: "/bin/zsh".into(),
///         arguments: vec![OsString::from("-c"), OsString::from("my-hook --check")],
///     }
/// );
/// ```
#[must_use]
pub fn plan(arguments: Vec<OsString>, shell: Option<OsString>) -> Plan {
    let mut arguments = arguments.into_iter();
    let Some(first) = arguments.next() else {
        return Plan::Hand {
            shell: real_shell(shell, None),
            arguments: Vec::new(),
        };
    };
    let others: Vec<OsString> = arguments.collect();

    let script = first.to_str().filter(|_| others.is_empty());
    let shell = real_shell(shell, script.and_then(snapshot));
    let shortened = script
        .and_then(asked)
        .filter(|command| !ShapeCompactor::keeps_whole(command));
    if let (Some(script), Some(command)) = (script, shortened) {
        return Plan::Shorten {
            shell,
            script: script.to_owned(),
            command,
        };
    }

    let mut handed = vec![OsString::from(COMMAND_FLAG), first];
    if !others.is_empty() {
        handed.push(OsString::from(NAME));
        handed.extend(others);
    }
    Plan::Hand {
        shell,
        arguments: handed,
    }
}

fn real_shell(named: Option<OsString>, sourced: Option<Snapshot>) -> PathBuf {
    let named = named.map(PathBuf::from).filter(|shell| shell.is_absolute());
    let kind_of = |shell: &Path| match shell.file_name()?.to_str()? {
        "bash" => Some(Snapshot::Bash),
        "zsh" => Some(Snapshot::Zsh),
        _ => None,
    };
    let named_kind = named.as_deref().and_then(kind_of);
    match (named, named_kind, sourced) {
        (Some(shell), Some(kind), wanted) if wanted.is_none_or(|wanted| wanted == kind) => shell,
        (_, _, Some(Snapshot::Bash)) => PathBuf::from(BASH),
        _ => PathBuf::from(ZSH),
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-shell/plan.rs"]
mod tests;
