mod answer;
mod arguments;
mod call;
mod project;

use std::process::Stdio;
use std::time::Duration;

use dot_api::Cancellation;

use self::arguments::{AGENT_LABEL, FINGERPRINT_LABEL, FOLDER_LABEL, Owner};
use crate::engine::{Engine, EngineError};
use crate::spec::{AgentName, AgentSpec};

pub use self::answer::{Answer, CallEvent};
pub use self::call::Call;
pub use self::project::{Project, ProjectError};

const QUICK: Duration = Duration::from_secs(30);
const CREATE: Duration = Duration::from_mins(15);
const STOP: Duration = Duration::from_mins(1);
const RUNNING: &str = "running";
const ABSENT: [&str; 5] = [
    "no such container",
    "no such object",
    "no such image",
    "no container with",
    "image not known",
];
const FIELD: char = '\t';

/// The containers the agents live in.
#[derive(Clone, Debug)]
pub struct Fleet {
    engine: Engine,
    owner: Owner,
}

/// A container of the fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its name, as the engine knows it.
    pub container: String,
    /// Its state, in the engine's words: `running`, `exited`…
    pub state: String,
    /// The agent that lives in it.
    pub agent: String,
    /// The project it works in.
    pub folder: String,
}

/// Why an agent did not answer.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
    /// The engine did not do what it was asked.
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// The container was made from another description of the agent. It is not replaced by
    /// itself: what the agent left in it would be lost.
    #[error(
        "{0} was described differently when its container was created: `dot-agents stop {0}`, then `dot-agents remove {0}`, and call it again"
    )]
    Outdated(AgentName),
    /// Dot's own image was never built.
    #[error("the image of the agents is not built yet: `dot-agents build`")]
    NotBuilt,
    /// The agent took longer than its description lets it.
    #[error("the agent did not answer within {} minutes", .0.as_secs() / 60)]
    TooLong(Duration),
    /// The call was stopped.
    #[error("the call was stopped")]
    Stopped,
    /// The call was to end, and neither it nor its container could be stopped.
    #[error("the call could not be ended: its agent may still be at work")]
    StillRunning(#[source] EngineError),
    /// The agent says it failed.
    #[error("the agent failed: {0}")]
    Failed(String),
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Absent,
    Present { running: bool, fingerprint: String },
}

impl Fleet {
    /// Creates the fleet `engine` runs, its containers run as the user. Nothing is started.
    #[must_use]
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            owner: Owner {
                user: rustix::process::getuid().as_raw(),
                group: rustix::process::getgid().as_raw(),
            },
        }
    }

    /// Has the agent live: creates its container for `project` when there is none, starts it
    /// when it sleeps, and returns its name.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine refuses, if the image is missing, or if the container was
    /// created from another description of the agent.
    pub fn start(
        &self,
        (name, spec): (&AgentName, &AgentSpec),
        project: &Project,
        cancellation: &Cancellation,
    ) -> Result<String, CallError> {
        let container = arguments::container_name(name, project);
        let run = (self.engine.kind(), self.owner);
        let mut state = self.state(&container, cancellation)?;
        if state == State::Absent {
            self.need_image(spec, cancellation)?;
            let create = arguments::create(&container, (name, spec), project, run);
            let created = self.engine.ask(create, CREATE, cancellation);
            state = self.state(&container, cancellation)?;
            if state == State::Absent {
                created?;
            }
        }
        match state {
            State::Present { fingerprint, .. }
                if fingerprint != arguments::fingerprint_of((name, spec), project, run) =>
            {
                Err(CallError::Outdated(name.clone()))
            }
            State::Present { running: true, .. } => Ok(container),
            State::Present { running: false, .. } | State::Absent => {
                self.engine
                    .ask(["start", &container], QUICK, cancellation)?;
                Ok(container)
            }
        }
    }

    /// Calls the agent: has it live, hands it the prompt, and waits for its answer.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent cannot live, fails, takes longer than its description lets
    /// it, or if `cancellation` is raised: what the call started in the container is then ended.
    pub fn call(
        &self,
        agent: (&AgentName, &AgentSpec),
        project: &Project,
        call: &Call<'_>,
        cancellation: &Cancellation,
        on_event: &mut dyn FnMut(CallEvent),
    ) -> Result<Answer, CallError> {
        let container = self.start(agent, project, cancellation)?;
        let spec = agent.1;
        call::run(
            &self.engine,
            &container,
            (spec, project),
            call,
            cancellation,
            on_event,
        )
    }

    /// Puts the agent to sleep: its container stops, and keeps what it holds.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine refuses, or the agent has no container for `project`.
    pub fn stop(
        &self,
        name: &AgentName,
        project: &Project,
        cancellation: &Cancellation,
    ) -> Result<(), EngineError> {
        let container = arguments::container_name(name, project);
        self.engine
            .ask(["stop", &container], STOP, cancellation)
            .map(drop)
    }

    /// Removes the container of an agent that sleeps. Its home (its sign-in, its conversations)
    /// is kept for the next one.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine refuses, as it does for a container that runs.
    pub fn remove(
        &self,
        name: &AgentName,
        project: &Project,
        cancellation: &Cancellation,
    ) -> Result<(), EngineError> {
        let container = arguments::container_name(name, project);
        self.engine
            .ask(["rm", &container], STOP, cancellation)
            .map(drop)
    }

    /// Signs the agent in to Claude, in the user's terminal; its home keeps the sign-in.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent cannot live, or if the sign-in does not complete.
    pub fn sign_in(
        &self,
        agent: (&AgentName, &AgentSpec),
        project: &Project,
        cancellation: &Cancellation,
    ) -> Result<(), CallError> {
        let container = self.start(agent, project, cancellation)?;
        let mut command = std::process::Command::new(self.engine.program());
        command
            .args(arguments::sign_in(&container))
            .stdin(Stdio::inherit());
        let mut child = self.engine.spawn(&mut command)?;
        let status = child.wait().map_err(EngineError::Lost)?;
        if status.success() {
            return Ok(());
        }
        Err(CallError::Failed(status.to_string()))
    }

    /// The containers of the fleet, for every project.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine does not answer.
    pub fn list(&self, cancellation: &Cancellation) -> Result<Vec<Listed>, EngineError> {
        let filter = format!("label={AGENT_LABEL}");
        let format = format!(
            "{{{{.Names}}}}\t{{{{.State}}}}\t{{{{.Label \"{AGENT_LABEL}\"}}}}\t{{{{.Label \"{FOLDER_LABEL}\"}}}}"
        );
        let arguments = ["ps", "--all", "--filter", &filter, "--format", &format];
        let printed = self.engine.ask(arguments, QUICK, cancellation)?;
        Ok(printed.lines().filter_map(listed).collect())
    }

    fn state(&self, container: &str, cancellation: &Cancellation) -> Result<State, EngineError> {
        let format =
            format!("{{{{.State.Status}}}}\t{{{{index .Config.Labels \"{FINGERPRINT_LABEL}\"}}}}");
        let arguments = [
            "inspect",
            "--type",
            "container",
            "--format",
            &format,
            container,
        ];
        match self.engine.ask(arguments, QUICK, cancellation) {
            Ok(printed) => {
                let (state, fingerprint) = printed
                    .trim()
                    .split_once(FIELD)
                    .unwrap_or((printed.trim(), ""));
                Ok(State::Present {
                    running: state == RUNNING,
                    fingerprint: fingerprint.to_owned(),
                })
            }
            Err(EngineError::Refused(why)) if tells_of_absence(&why) => Ok(State::Absent),
            Err(error) => Err(error),
        }
    }

    fn need_image(&self, spec: &AgentSpec, cancellation: &Cancellation) -> Result<(), CallError> {
        if !spec.uses_the_default_image() {
            return Ok(());
        }
        let arguments = ["image", "inspect", "--format", "{{.Id}}", &spec.image];
        match self.engine.ask(arguments, QUICK, cancellation) {
            Ok(_) => Ok(()),
            Err(EngineError::Refused(why)) if tells_of_absence(&why) => Err(CallError::NotBuilt),
            Err(error) => Err(error.into()),
        }
    }
}

fn tells_of_absence(why: &str) -> bool {
    let why = why.to_ascii_lowercase();
    ABSENT.iter().any(|absent| why.contains(absent))
}

fn listed(line: &str) -> Option<Listed> {
    let mut fields = line.splitn(4, FIELD).map(str::to_owned);
    Some(Listed {
        container: fields.next()?,
        state: fields.next()?,
        agent: fields.next()?,
        folder: fields.next()?,
    })
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/fleet/mod.rs"]
pub(crate) mod tests;
