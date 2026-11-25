mod message;

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, sync_channel};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use dot_api::Cancellation;
use dot_process::{Line, read_line};
use serde_json::{Value, json};

use self::message::{Asked, CallArguments, Request, failure, success, tool_answer, tools};
use crate::fleet::{Call, Fleet, Project};
use crate::roster::Roster;
use crate::spec::AgentName;

const LONGEST_MESSAGE: usize = 4 * 1_024 * 1_024;
const INVALID_REQUEST: i32 = -32_600;
const METHOD_NOT_FOUND: i32 = -32_601;
const INVALID_PARAMETERS: i32 = -32_602;
const PARSE_ERROR: i32 = -32_700;
const QUEUED: usize = 16;
const LOOK: Duration = Duration::from_millis(100);
const NEW_LINE: &[u8] = b"\n";

enum Received {
    Message(Vec<u8>),
    TooLong,
}

struct Server<'a, W> {
    fleet: &'a Fleet,
    agents: &'a Path,
    project: &'a Project,
    output: Mutex<W>,
    calls: Mutex<HashMap<String, Cancellation>>,
}

/// Serves the calls to agents over the Model Context Protocol, one message per line, until
/// `input` ends or `cancellation` is raised; every call still going on is then stopped.
///
/// Claude finds two tools: `list_agents`, the agents described in the folder `agents`, read again
/// each time; and `call_agent`, which calls one in `project`. Several calls go on at once.
///
/// # Errors
///
/// Returns an error if `input` cannot be read.
pub fn serve(
    fleet: &Fleet,
    agents: &Path,
    project: &Project,
    (input, output): (impl BufRead + Send + 'static, impl Write + Send),
    cancellation: &Cancellation,
) -> io::Result<()> {
    let server = Server {
        fleet,
        agents,
        project,
        output: Mutex::new(output),
        calls: Mutex::default(),
    };
    let lines = lines(input);
    thread::scope(|scope| {
        let ended = server.follow(&lines, cancellation, scope);
        server.stop_all();
        ended
    })
}

/// Reads from a thread of its own, which ends with the input: waiting for a message never keeps a
/// stop from being followed.
fn lines(mut input: impl BufRead + Send + 'static) -> Receiver<io::Result<Received>> {
    let (to, lines) = sync_channel(QUEUED);
    thread::spawn(move || {
        loop {
            let mut line = Vec::new();
            let received = match read_line(&mut input, &mut line, LONGEST_MESSAGE) {
                Ok(None) => return,
                Ok(Some(Line::Whole)) => Ok(Received::Message(line)),
                Ok(Some(Line::Cut)) => Ok(Received::TooLong),
                Err(error) => Err(error),
            };
            let over = received.is_err();
            if to.send(received).is_err() || over {
                return;
            }
        }
    });
    lines
}

impl<W: Write + Send> Server<'_, W> {
    fn follow<'scope>(
        &'scope self,
        lines: &Receiver<io::Result<Received>>,
        cancellation: &Cancellation,
        scope: &'scope thread::Scope<'scope, '_>,
    ) -> io::Result<()> {
        while !cancellation.is_cancelled() {
            match lines.recv_timeout(LOOK) {
                Ok(Ok(Received::Message(line))) => self.hear(&line, scope),
                Ok(Ok(Received::TooLong)) => {
                    self.send(&failure(&Value::Null, INVALID_REQUEST, "too long"));
                }
                Ok(Err(error)) => return Err(error),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
        Ok(())
    }

    fn hear<'scope>(&'scope self, line: &[u8], scope: &'scope thread::Scope<'scope, '_>) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let Ok(request) = serde_json::from_slice::<Request>(line) else {
            self.send(&failure(&Value::Null, PARSE_ERROR, "not a message"));
            return;
        };
        let Request { id, asked } = request;
        let Some(id) = id else {
            if let Asked::Cancelled(id) = asked {
                self.stop(&id);
            }
            return;
        };
        match asked {
            Asked::Initialize(version) => self.send(&success(&id, &message::welcome(&version))),
            Asked::Ping => self.send(&success(&id, &json!({}))),
            Asked::ListTools => self.send(&success(&id, &tools())),
            Asked::ListAgents => self.send(&success(&id, &self.list_agents())),
            Asked::CallAgent(arguments) => {
                let cancellation = Cancellation::new();
                let key = id.to_string();
                self.calls().insert(key.clone(), cancellation.clone());
                scope.spawn(move || {
                    let answered = self.call_agent(&arguments, &cancellation);
                    self.calls().remove(&key);
                    if !cancellation.is_cancelled() {
                        self.send(&success(&id, &answered));
                    }
                });
            }
            Asked::Malformed(why) => self.send(&failure(&id, INVALID_PARAMETERS, &why)),
            Asked::Cancelled(_) | Asked::Unknown => {
                self.send(&failure(&id, METHOD_NOT_FOUND, "unknown method"));
            }
        }
    }

    fn list_agents(&self) -> Value {
        match Roster::read(self.agents) {
            Err(error) => tool_answer(&described(&error), true),
            Ok(roster) => {
                let agents: Vec<Value> = roster
                    .agents()
                    .map(|(name, spec)| {
                        json!({"agent": name.as_str(), "description": spec.description})
                    })
                    .collect();
                let refused: Vec<String> = roster
                    .refused()
                    .iter()
                    .map(|error| described(error))
                    .collect();
                let told = json!({"agents": agents, "unusable": refused});
                tool_answer(&told.to_string(), false)
            }
        }
    }

    fn call_agent(&self, arguments: &CallArguments, cancellation: &Cancellation) -> Value {
        let answered = AgentName::parse(&arguments.agent)
            .map_err(|error| described(&error))
            .and_then(|name| {
                let roster = Roster::read(self.agents).map_err(|error| described(&error))?;
                let spec = roster
                    .get(&name)
                    .ok_or_else(|| format!("no agent is called {name}"))?;
                let call = Call {
                    prompt: &arguments.prompt,
                    goes_on: arguments.goes_on,
                };
                self.fleet
                    .call(
                        (&name, spec),
                        self.project,
                        &call,
                        cancellation,
                        &mut |_| {},
                    )
                    .map_err(|error| described(&error))
            });
        match answered {
            Ok(answer) => tool_answer(&answer.text, false),
            Err(why) => tool_answer(&why, true),
        }
    }

    fn stop(&self, id: &Value) {
        if let Some(cancellation) = self.calls().get(&id.to_string()) {
            cancellation.cancel();
        }
    }

    fn stop_all(&self) {
        self.calls().values().for_each(Cancellation::cancel);
    }

    fn calls(&self) -> MutexGuard<'_, HashMap<String, Cancellation>> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A message nobody reads any more is no error: the input ends right after.
    fn send(&self, message: &Value) {
        let mut line = message.to_string().into_bytes();
        line.extend_from_slice(NEW_LINE);
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        drop(output.write_all(&line).and_then(|()| output.flush()));
    }
}

fn described(error: &dyn std::error::Error) -> String {
    let causes = std::iter::successors(error.source(), |&cause| cause.source());
    causes.fold(error.to_string(), |told, cause| format!("{told}: {cause}"))
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/mcp/mod.rs"]
mod tests;
