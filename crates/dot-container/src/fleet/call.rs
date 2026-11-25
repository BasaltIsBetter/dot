use std::hash::BuildHasher;
use std::io::{BufReader, Read};
use std::process::{Child, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Cancellation;
use dot_process::{Waited, read_line};

use crate::engine::run::{heard, listen, refusal};
use crate::engine::{Engine, EngineError, hand_over};
use crate::fleet::CallError;
use crate::fleet::answer::{Answer, CallEvent, Reader};
use crate::fleet::arguments;
use crate::fleet::project::Project;
use crate::spec::AgentSpec;

const LONGEST_LINE: usize = 4 * 1_024 * 1_024;
const QUEUED: usize = 64;
const LOOK: Duration = Duration::from_millis(50);
const END_IN_TIME: Duration = Duration::from_secs(10);
const ASK_TO_END: &str = "TERM";
const END: &str = "KILL";
const GRACE: Duration = Duration::from_secs(1);

/// What an agent is called for.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    /// What it is asked.
    pub prompt: &'a str,
    /// Whether it goes on with its last conversation in this project, or starts a new one.
    pub goes_on: bool,
}

enum Followed {
    ToTheEnd,
    UntilTooLate,
    UntilStopped,
}

pub(crate) fn run(
    engine: &Engine,
    container: &str,
    (spec, project): (&AgentSpec, &Project),
    call: &Call<'_>,
    cancellation: &Cancellation,
    on_event: &mut dyn FnMut(CallEvent),
) -> Result<Answer, CallError> {
    let mark = mark();
    let mut command = engine.command(arguments::call(
        container,
        spec,
        project,
        (&mark, call.goes_on),
    ));
    command.stdin(Stdio::piped());
    for variable in &spec.env {
        if let Some(value) = std::env::var_os(variable) {
            command.env(variable, value);
        }
    }
    let mut child = engine.spawn(&mut command)?;
    if let Some(to) = child.stdin.take() {
        hand_over(call.prompt.to_owned(), to);
    }
    let errors = child.stderr.take().map(listen);
    let lines = child.stdout.take().map(lines);
    let mut reader = Reader::of(spec.command.is_empty());

    let limit = Duration::from_secs(u64::from(spec.minutes) * 60);
    let deadline = Instant::now() + limit;
    let followed = follow(lines.as_ref(), deadline, cancellation, &mut |line| {
        reader.hear(&line, on_event);
    });
    drop(lines);
    let ending = match followed {
        Followed::ToTheEnd => dot_process::wait(&mut child, Some(deadline), cancellation)
            .map_err(EngineError::Lost)?,
        Followed::UntilTooLate => Waited::TimedOut,
        Followed::UntilStopped => Waited::Cancelled,
    };
    let status = match ending {
        Waited::Exited(status) => status,
        unfinished => {
            end(engine, container, &mark, &mut child)?;
            return Err(match unfinished {
                Waited::Cancelled => CallError::Stopped,
                _ => CallError::TooLong(limit),
            });
        }
    };

    let errors = heard(errors)?;
    match reader.answer() {
        Some(Err(said)) => Err(CallError::Failed(said)),
        Some(Ok(answer)) if status.success() => Ok(answer),
        Some(Ok(_)) | None => Err(CallError::Failed(refusal(&errors, "", status))),
    }
}

fn follow(
    lines: Option<&Receiver<Vec<u8>>>,
    deadline: Instant,
    cancellation: &Cancellation,
    on_line: &mut dyn FnMut(Vec<u8>),
) -> Followed {
    let Some(lines) = lines else {
        return Followed::ToTheEnd;
    };
    loop {
        if cancellation.is_cancelled() {
            return Followed::UntilStopped;
        }
        if Instant::now() >= deadline {
            return Followed::UntilTooLate;
        }
        match lines.recv_timeout(LOOK) {
            Ok(line) => on_line(line),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Followed::ToTheEnd,
        }
    }
}

/// Ends what the call started inside the container first: ending the engine's client alone
/// leaves it running there.
fn end(engine: &Engine, container: &str, mark: &str, child: &mut Child) -> Result<(), EngineError> {
    let on = Cancellation::new();
    let ask = |signal| {
        engine
            .ask(
                arguments::end_call(container, mark, signal),
                END_IN_TIME,
                &on,
            )
            .map(drop)
    };
    let asked = ask(ASK_TO_END);
    let waited = dot_process::wait(child, Some(Instant::now() + GRACE), &on);
    let ended = ask(END);
    let stopped = dot_process::stop(child).map_err(EngineError::Lost);
    waited.map_err(EngineError::Lost)?;
    asked.and(ended).and(stopped)
}

fn lines(from: impl Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (to, lines) = sync_channel(QUEUED);
    thread::spawn(move || pass_on(from, &to));
    lines
}

fn pass_on(from: impl Read, to: &SyncSender<Vec<u8>>) {
    let mut from = BufReader::new(from);
    let mut line = Vec::new();
    while let Ok(Some(_)) = read_line(&mut from, &mut line, LONGEST_LINE) {
        if to.send(std::mem::take(&mut line)).is_err() {
            return;
        }
    }
}

fn mark() -> String {
    let random = std::hash::RandomState::new();
    let first = random.hash_one(std::process::id());
    let second = random.hash_one(first);
    format!("{first:016x}{second:016x}")
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/fleet/call.rs"]
mod tests;
