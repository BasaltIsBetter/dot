use std::fmt;

use serde::Deserialize;
use serde_json::{Map, Value};

const LONGEST_NAME: usize = 32;
const NAME_DASH: u8 = b'-';
const DEFAULT_IMAGE: &str = "dot-agent:latest";
const DEFAULT_MEMORY: &str = "2g";
const DEFAULT_PROCESSES: u32 = 512;
const DEFAULT_PROCESSORS: f64 = 2.0;
const MOST_PROCESSORS: f64 = 1_024.0;
const DEFAULT_MINUTES: u32 = 30;
const MOST_MINUTES: u32 = 24 * 60;
const OPTION_PREFIX: char = '-';
const SIZE_UNITS: [char; 4] = ['b', 'k', 'm', 'g'];

/// The name of an agent: lowercase letters, digits and inner dashes, 32 characters at most.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgentName(String);

/// What an agent may do with the project's folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// It does not see it.
    None,
    /// It reads it.
    #[default]
    Read,
    /// It reads and changes it.
    Write,
}

/// What an agent reaches of the network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// The network, from behind the engine's bridge: Claude answers from there.
    #[default]
    Bridge,
    /// Nothing at all, for an agent that runs a program of its image.
    None,
}

/// An agent, as its file describes it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AgentSpec {
    /// What the agent is for, as Claude reads it to choose whom to call.
    pub description: String,
    /// The image its container is made from.
    pub image: String,
    /// What it may do with the project's folder.
    pub workspace: Access,
    /// What it reaches of the network.
    pub network: Network,
    /// The memory it may take, such as `2g` or `512m`.
    pub memory: String,
    /// The processors it may use at once.
    pub cpus: f64,
    /// The processes it may run at once.
    pub processes: u32,
    /// How long one call may take, in minutes.
    pub minutes: u32,
    /// The model Claude Code uses in it; its own choice when left out.
    pub model: Option<String>,
    /// What the agent is told before each prompt.
    pub instructions: String,
    /// The program that answers instead of Claude Code: it reads the prompt on its input, and
    /// prints the answer.
    pub command: Vec<String>,
    /// The variables of Dot's environment the agent sees, by name.
    pub env: Vec<String>,
}

/// What is wrong in an agent's name or file.
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    /// The name is not one an agent can have.
    #[error(
        "{0:?} cannot name an agent: use lowercase letters, digits and inner dashes, 32 at most"
    )]
    Name(String),
    /// The file is not JSON, or not that of an agent.
    #[error("not the description of an agent")]
    Json(#[from] serde_json::Error),
    /// A field holds what it cannot.
    #[error("{0}: {1}")]
    Field(&'static str, &'static str),
}

impl AgentName {
    /// Reads a name.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError::Name`] if `text` cannot name an agent.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_container::AgentName;
    ///
    /// assert!(AgentName::parse("reviewer-2").is_ok());
    /// assert!(AgentName::parse("../reviewer").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, SpecError> {
        let plain = |byte: &u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let bytes = text.as_bytes();
        let well_formed = bytes.len() <= LONGEST_NAME
            && bytes.first().is_some_and(plain)
            && bytes.last().is_some_and(plain)
            && bytes.iter().all(|byte| plain(byte) || *byte == NAME_DASH);
        if !well_formed {
            return Err(SpecError::Name(text.to_owned()));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Default for AgentSpec {
    fn default() -> Self {
        Self {
            description: String::new(),
            image: DEFAULT_IMAGE.to_owned(),
            workspace: Access::default(),
            network: Network::default(),
            memory: DEFAULT_MEMORY.to_owned(),
            cpus: DEFAULT_PROCESSORS,
            processes: DEFAULT_PROCESSES,
            minutes: DEFAULT_MINUTES,
            model: None,
            instructions: String::new(),
            command: Vec::new(),
            env: Vec::new(),
        }
    }
}

impl AgentSpec {
    /// Reads the description of an agent from its JSON. A field left out keeps its default; one
    /// that does not exist is an error, so that a misspelt limit never goes unnoticed.
    ///
    /// # Errors
    ///
    /// Returns an error if `json` is not the description of an agent, or holds what a container
    /// cannot have.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_container::{Access, AgentSpec};
    ///
    /// let spec = AgentSpec::parse(br#"{"description": "Reviews", "memory": "1g"}"#).unwrap();
    /// assert_eq!(spec.workspace, Access::Read);
    /// assert!(AgentSpec::parse(br#"{"privileged": true}"#).is_err());
    /// ```
    pub fn parse(json: &[u8]) -> Result<Self, SpecError> {
        let fields: Map<String, Value> = serde_json::from_slice(json)?;
        let spec: Self = serde_json::from_value(Value::Object(fields))?;
        spec.check()?;
        Ok(spec)
    }

    /// Whether the image is Dot's own, which `dot-agents build` builds.
    #[must_use]
    pub fn uses_the_default_image(&self) -> bool {
        self.image == DEFAULT_IMAGE
    }

    /// The image `dot-agents build` builds.
    #[must_use]
    pub const fn default_image() -> &'static str {
        DEFAULT_IMAGE
    }

    fn check(&self) -> Result<(), SpecError> {
        let refuse = |field, why| Err(SpecError::Field(field, why));
        if !is_an_image(&self.image) {
            return refuse("image", "not the name of an image");
        }
        if !is_a_size(&self.memory) {
            return refuse("memory", "write a number and b, k, m or g, such as 2g");
        }
        if !(self.cpus > 0.0 && self.cpus <= MOST_PROCESSORS) {
            return refuse("cpus", "more than 0, and 1024 at most");
        }
        if self.processes == 0 {
            return refuse("processes", "at least 1");
        }
        if self.minutes == 0 || self.minutes > MOST_MINUTES {
            return refuse("minutes", "from 1 minute to 24 hours");
        }
        if !self.env.iter().all(|name| is_a_variable(name)) {
            return refuse("env", "the names of variables, without their values");
        }
        if self
            .model
            .as_deref()
            .is_some_and(|model| !is_a_model(model))
        {
            return refuse("model", "not the name of a model");
        }
        if self.command.first().is_some_and(String::is_empty) {
            return refuse("command", "the program has no name");
        }
        Ok(())
    }
}

fn is_an_image(image: &str) -> bool {
    !image.is_empty()
        && !image.starts_with(OPTION_PREFIX)
        && image
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-/:@".contains(character))
}

fn is_a_size(size: &str) -> bool {
    let digits = size.trim_end_matches(SIZE_UNITS);
    size.len() <= digits.len() + 1
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && digits.parse::<u64>().is_ok_and(|size| size > 0)
}

fn is_a_variable(name: &str) -> bool {
    name.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_a_model(model: &str) -> bool {
    !model.is_empty()
        && !model.starts_with(OPTION_PREFIX)
        && !model
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

#[cfg(test)]
#[path = "../../../tests/dot-container/spec.rs"]
mod tests;
