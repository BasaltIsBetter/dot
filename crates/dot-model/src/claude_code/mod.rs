mod control;
mod error;
mod launch;
mod listen;

#[cfg(feature = "bench")]
pub(crate) use self::listen::{Incoming, QUEUED, read_lines};
#[cfg(feature = "bench")]
pub(crate) use self::output::parse;
mod output;
mod summary;
mod tools;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dot_api::{
    Effort, Instructions, Model, Prompt, Reading, Reply, ReplyError, ReplyEvent, ReplyFailure,
    Summaries, Toolbox, Tuning,
};
use dot_process::{LiveOutputs, LoginPath};

use crate::claude_code::error::{ClaudeCodeError, failed, no_result};
use crate::claude_code::launch::{
    AgentServer, Handed, Options, Running, SessionId, command, hand_over, handed,
};
use crate::claude_code::listen::{Heard, Listener, live_outputs};
use crate::claude_code::output::Compaction;
use crate::context::Ledger;
use crate::messages::Hands;
use crate::pricing;

pub use self::launch::summaries_for;

pub(crate) const EXIT_GRACE: Duration = Duration::from_secs(5);
pub(crate) const NEW_LINE: &[u8] = b"\n";

pub(crate) struct ClaudeCodeChat {
    program: Option<PathBuf>,
    workspace: Option<PathBuf>,
    shell: Option<PathBuf>,
    login_path: LoginPath,
    model: Model,
    effort: Effort,
    summaries: Summaries,
    allowed: Summaries,
    reading: Reading,
    instructions: Instructions,
    agents: Option<AgentServer>,
    session: Option<SessionId>,
    ledger: Ledger,
    unannounced: Option<Compaction>,
    billed: u64,
}

impl ClaudeCodeChat {
    pub(crate) fn new(
        model: Model,
        effort: Effort,
        program: Option<PathBuf>,
        workspace: Option<PathBuf>,
        shell: Option<PathBuf>,
    ) -> Self {
        Self {
            program,
            workspace,
            shell,
            login_path: LoginPath::default(),
            model,
            effort,
            summaries: Summaries::LeftToClaude,
            allowed: Summaries::WhenTheyPay,
            reading: Reading::ByClaude,
            instructions: Instructions::LoadedByClaudeCode,
            agents: None,
            session: None,
            ledger: Ledger::NOTHING_YET,
            unannounced: None,
            billed: 0,
        }
    }

    pub(crate) const fn reading(mut self, reading: Reading) -> Self {
        self.reading = reading;
        self
    }

    pub(crate) const fn instructions(mut self, instructions: Instructions) -> Self {
        self.instructions = instructions;
        self
    }

    pub(crate) fn agents(mut self, program: PathBuf, described_in: PathBuf) -> Self {
        self.agents = Some(AgentServer {
            program,
            described_in,
        });
        self
    }

    pub(crate) const fn summarizing(mut self, summaries: Summaries) -> Self {
        self.summaries = summaries;
        self
    }

    pub(crate) const fn allowing(mut self, allowed: Summaries) -> Self {
        self.allowed = allowed;
        self
    }

    pub(crate) const fn tune(&mut self, tuning: Tuning) {
        self.model = tuning.model;
        self.effort = tuning.effort;
        self.summaries = tuning.summaries;
    }

    const fn summaries(&self) -> Summaries {
        self.summaries.within(self.allowed)
    }

    pub(crate) fn login_path(mut self, login_path: LoginPath) -> Self {
        self.login_path = login_path;
        self
    }

    pub(crate) fn reply<T: Toolbox>(
        &mut self,
        prompt: Prompt,
        hands: &Hands<'_, T>,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        self.announce_summary(on_event);
        let cancellation = hands.cancellation;
        let program = self
            .program
            .as_deref()
            .ok_or_else(|| failed(ClaudeCodeError::Missing))?;
        let folder = self.folder(hands.toolbox.folder())?;

        let live = self
            .shell
            .as_ref()
            .and_then(|_| live_outputs(self.workspace.as_deref()));
        let handed = self.handed(folder);
        let agents = self.agents_of(hands.toolbox.folder());
        let told = (handed.as_ref(), agents.as_deref());
        let options = self.options(told, self.summary_window(), live.as_ref());
        let launch = command(program, folder, self.model.id, self.effort, options);
        let (running, pipes) = Running::start(launch, cancellation).map_err(failed)?;

        let mut stdin = pipes.stdin;
        let handed = hand_over(stdin.as_mut(), &prompt);
        drop(prompt);
        let (heard, status) = Listener {
            session: &mut self.session,
            ledger: &mut self.ledger,
            stdin: &mut stdin,
            hands,
            on_event,
        }
        .listen_to(pipes.stdout, live.as_ref(), &running);
        let status = status.map_err(failed)?;
        if cancellation.is_cancelled() {
            let spent = heard
                .as_ref()
                .map_or_else(|_| Reply::default(), |heard| self.spent_by(heard));
            return Err(ReplyError::new(ReplyFailure::Stopped).with_spent(spent));
        }

        let heard = heard.map_err(failed)?;
        let spent = self.spent_by(&heard);
        let Some(outcome) = &heard.outcome else {
            if !heard.understood_a_line {
                self.session = None;
            }
            let stopped = no_result(status, &running.said());
            let error = handed.map_or_else(failed, |()| failed(stopped));
            return Err(error.with_spent(spent));
        };
        if let Some(failure) = outcome.failure() {
            return Err(ReplyError::new(failure).with_spent(spent));
        }
        if heard.turn.is_declined() {
            return Err(ReplyError::new(ReplyFailure::Declined).with_spent(spent));
        }
        Ok(spent)
    }

    pub(crate) fn reset(&mut self) {
        self.session = None;
        self.ledger = Ledger::NOTHING_YET;
        self.unannounced = None;
        self.billed = 0;
    }

    fn options<'a>(
        &'a self,
        (handed, agents): (Option<&'a Handed>, Option<&'a str>),
        summary_window: Option<u64>,
        live: Option<&'a LiveOutputs>,
    ) -> Options<'a> {
        Options {
            session: self.session.as_ref(),
            shell: self.shell.as_deref(),
            live: live.map(LiveOutputs::address),
            path: self.login_path.read(),
            summary_window,
            reading: self.reading,
            instructions: handed,
            agents,
        }
    }

    /// Agents work in the user's project, never in Dot's own folder.
    fn agents_of(&self, project: Option<&Path>) -> Option<String> {
        self.agents.as_ref()?.configuration(project?)
    }

    fn handed(&self, folder: &Path) -> Option<Handed> {
        match self.instructions {
            Instructions::LoadedByClaudeCode => None,
            Instructions::HandedByDot => handed(folder),
        }
    }

    fn spent_by(&mut self, heard: &Heard<'_>) -> Reply {
        let mut spent = heard.spent();
        if heard.outcome.is_some() {
            spent.cost = pricing::billed_since(&mut self.billed, spent.cost);
        }
        spent
    }
}

impl ClaudeCodeChat {
    fn folder<'a>(&'a self, project: Option<&'a Path>) -> Result<&'a Path, ReplyError> {
        if let Some(project) = project {
            return Ok(project);
        }
        let workspace = self
            .workspace
            .as_deref()
            .ok_or_else(|| failed(ClaudeCodeError::NoWorkspace))?;
        fs::create_dir_all(workspace).map_err(failed)?;
        Ok(workspace)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/mod.rs"]
pub(crate) mod tests;
