use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::spec::{AgentName, AgentSpec, SpecError};

const EXTENSION: &str = "json";
const LARGEST_FILE: u64 = 256 * 1_024;
const TOO_LARGE: &str = "larger than 256 KiB";
const NOT_A_FILE: &str = "not a plain file";
const NAMELESS: &str = "its name is not text";
#[cfg(target_os = "macos")]
const FOLDER: &str = "Library/Application Support/Dot/agents";
#[cfg(not(target_os = "macos"))]
const FOLDER: &str = ".config/dot/agents";
#[cfg(not(target_os = "macos"))]
const CONFIGURATION_VARIABLE: &str = "XDG_CONFIG_HOME";
#[cfg(not(target_os = "macos"))]
const FOLDER_IN_CONFIGURATION: &str = "dot/agents";
const TEMPLATE: &str = r#"{
  "description": "Reviews the code of the project and reports its defects.",
  "workspace": "read",
  "network": "bridge",
  "memory": "2g",
  "cpus": 2,
  "minutes": 30,
  "instructions": "You review code. Report each defect with its file and line. Change nothing."
}
"#;

/// The agents of the user, by name: the JSON files of one folder.
#[derive(Debug, Default)]
pub struct Roster {
    agents: BTreeMap<AgentName, AgentSpec>,
    refused: Vec<RosterError>,
}

/// What keeps an agent from being used.
#[derive(Debug, thiserror::Error)]
pub enum RosterError {
    /// The folder or a file cannot be read or written.
    #[error("{}", .0.display())]
    File(PathBuf, #[source] io::Error),
    /// A file does not describe an agent.
    #[error("{}", .0.display())]
    Spec(PathBuf, #[source] SpecError),
}

/// Returns the folder where the user's agents are described.
///
/// On Linux, `dot/agents` in `XDG_CONFIG_HOME`, or in `~/.config`; on the Mac, in Dot's
/// Application Support folder. `variable` reads a variable of the environment.
pub fn agents_folder(home: &Path, variable: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    #[cfg(not(target_os = "macos"))]
    if let Some(configuration) = variable(CONFIGURATION_VARIABLE)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return configuration.join(FOLDER_IN_CONFIGURATION);
    }
    #[cfg(target_os = "macos")]
    drop(variable);
    home.join(FOLDER)
}

/// Returns whether `folder` describes an agent at all, without reading any.
#[must_use]
pub fn has_agents(folder: &Path) -> bool {
    files(folder).is_ok_and(|files| !files.is_empty())
}

impl Roster {
    /// Reads the agents described in `folder`. A missing folder describes none; a file that
    /// cannot be used is set aside, among [`Roster::refused`], and leaves the others usable.
    ///
    /// # Errors
    ///
    /// Returns an error if `folder` exists and cannot be read.
    pub fn read(folder: &Path) -> Result<Self, RosterError> {
        let mut roster = Self::default();
        let files = files(folder).map_err(|error| RosterError::File(folder.to_owned(), error))?;
        for path in files {
            match agent(&path) {
                Ok((name, spec)) => {
                    roster.agents.insert(name, spec);
                }
                Err(error) => roster.refused.push(error),
            }
        }
        Ok(roster)
    }

    /// Writes a first description of the agent `name` in `folder`, and returns its path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written, or if the agent is described already.
    pub fn describe(folder: &Path, name: &AgentName) -> Result<PathBuf, RosterError> {
        let path = folder.join(name.as_str()).with_extension(EXTENSION);
        let write = || {
            fs::create_dir_all(folder)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(TEMPLATE.as_bytes())
        };
        write().map_err(|error| RosterError::File(path.clone(), error))?;
        Ok(path)
    }

    /// The agents, in the order of their names.
    pub fn agents(&self) -> impl Iterator<Item = (&AgentName, &AgentSpec)> {
        self.agents.iter()
    }

    /// The agent called `name`.
    #[must_use]
    pub fn get(&self, name: &AgentName) -> Option<&AgentSpec> {
        self.agents.get(name)
    }

    /// The files set aside, and why.
    #[must_use]
    pub fn refused(&self) -> &[RosterError] {
        &self.refused
    }
}

fn files(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == EXTENSION)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn agent(path: &Path) -> Result<(AgentName, AgentSpec), RosterError> {
    let in_file = |error| RosterError::File(path.to_owned(), error);
    let in_spec = |error| RosterError::Spec(path.to_owned(), error);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| in_file(io::Error::other(NAMELESS)))?;
    let name = AgentName::parse(stem).map_err(in_spec)?;
    let json = read(path).map_err(in_file)?;
    let spec = AgentSpec::parse(&json).map_err(in_spec)?;
    Ok((name, spec))
}

fn read(path: &Path) -> io::Result<Vec<u8>> {
    let (file, metadata) = dot_process::open_without_waiting(path)?;
    if !metadata.is_file() {
        return Err(io::Error::other(NOT_A_FILE));
    }
    let mut written = Vec::new();
    file.take(LARGEST_FILE + 1).read_to_end(&mut written)?;
    if u64::try_from(written.len()).is_ok_and(|length| length > LARGEST_FILE) {
        return Err(io::Error::other(TOO_LARGE));
    }
    Ok(written)
}

#[cfg(test)]
#[path = "../../../tests/dot-container/roster.rs"]
mod tests;
