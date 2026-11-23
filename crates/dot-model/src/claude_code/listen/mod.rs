mod agents;
mod channel;
mod live;

#[cfg(feature = "bench")]
pub(crate) use self::channel::read_lines;

use std::io;
use std::path::Path;
use std::process::ChildStdin;
use std::sync::mpsc::Receiver;

use dot_api::{Decision, Limits, Reply, ReplyEvent, Toolbox};

use crate::claude_code::control::ControlRequest;
use crate::claude_code::launch::SessionId;
use crate::claude_code::output::{Compaction, Line, Outcome, parse};
use crate::context::{Ledger, Request};
use crate::messages::Hands;
use crate::pricing;
use crate::stream::{Event, Turn};

use self::agents::{Agents, end_tools};
pub(crate) use self::channel::Incoming;
use self::live::LiveCommands;

pub(crate) use self::live::live_outputs;

pub(crate) const LONGEST_LINE: usize = 16 * 1_024 * 1_024;

pub(crate) struct Heard<'a> {
    pub(crate) turn: Turn<'a>,
    pub(crate) base_tokens: Option<u64>,
    pub(crate) limits: Limits,
    pub(crate) outcome: Option<Outcome>,
}

pub(crate) struct Listener<'l, 'h, T> {
    pub(crate) session: &'l mut Option<SessionId>,
    pub(crate) ledger: &'l mut Ledger,
    pub(crate) stdin: &'l mut Option<ChildStdin>,
    pub(crate) hands: &'h Hands<'h, T>,
    pub(crate) on_event: &'l mut dyn FnMut(ReplyEvent),
}

impl<'a> Heard<'a> {
    pub(crate) fn nothing(folder: Option<&'a Path>) -> Self {
        Self {
            turn: Turn::listening(folder),
            base_tokens: None,
            limits: Limits::default(),
            outcome: None,
        }
    }

    pub(crate) fn spent(&self) -> Reply {
        let usage = self.turn.usage();
        let cost = self
            .outcome
            .as_ref()
            .map_or_else(|| pricing::cost(self.turn.model(), usage), Outcome::cost);
        Reply {
            context_tokens: usage.context_tokens(),
            base_tokens: self.base_tokens,
            output_tokens: self.turn.output_tokens(),
            cost,
            limits: self.limits,
        }
    }
}

impl<'h, T: Toolbox> Listener<'_, 'h, T> {
    pub(crate) fn listen(&mut self, incoming: &Receiver<Incoming>) -> io::Result<Heard<'h>> {
        let folder = self.hands.toolbox.folder();
        let mut heard = Heard::nothing(folder);
        let mut refused = Vec::new();
        let mut agents = Agents::default();
        let mut commands = LiveCommands::new(self.hands.compactor);
        let starts_a_conversation = self.session.is_none();

        for arrived in incoming {
            let line = match arrived {
                Incoming::Line(line) => line,
                Incoming::Piece {
                    connection,
                    command,
                    bytes,
                } => {
                    if let Some(event) = commands.heard(connection, &command, &bytes) {
                        (self.on_event)(event);
                    }
                    continue;
                }
                Incoming::Ended(ended) => {
                    ended?;
                    break;
                }
            };
            let on_event = &mut *self.on_event;
            let mut emit = |event: ReplyEvent| {
                commands.notice(&event);
                on_event(event);
            };
            match parse(&line) {
                Ok(Line::System {
                    session_id,
                    compaction,
                }) => {
                    if let Some(id) = session_id.as_deref().and_then(SessionId::parse) {
                        *self.session = Some(id);
                    }
                    if let Some(Compaction { before, after }) = compaction {
                        self.ledger.summarized();
                        emit(ReplyEvent::Compacted { before, after });
                    }
                }
                Ok(Line::Stream(event)) => {
                    let opens_a_request = matches!(event, Event::MessageStart { .. });
                    heard.turn.apply(event, &mut emit);
                    if !opens_a_request {
                        continue;
                    }
                    let usage = heard.turn.usage();
                    self.ledger.request(Request {
                        read: usage.prompt_tokens(),
                        written: usage.cache_written(),
                        written_for_an_hour: usage.cache_written_for_an_hour(),
                    });
                    if starts_a_conversation && heard.base_tokens.is_none() {
                        heard.base_tokens = read_tokens(usage.prompt_tokens());
                    }
                }
                Ok(Line::RateLimits(limits)) => heard.limits = limits,
                Ok(Line::ToolResults {
                    agent: of_agent,
                    results,
                }) => {
                    let ended = (of_agent, refused.as_slice(), folder);
                    end_tools(results, ended, &mut agents, &mut emit);
                }
                Ok(Line::Agent(line)) => agents.hear(line, folder, &mut emit),
                Ok(Line::Control(request)) => self.answer(&request, &mut refused),
                Ok(Line::Result(outcome)) => {
                    heard.outcome = Some(outcome);
                    break;
                }
                Ok(Line::Other) | Err(_) => {}
            }
        }
        Ok(heard)
    }

    fn answer(&mut self, request: &ControlRequest, refused: &mut Vec<String>) {
        let permission = request.permission(&*self.hands.toolbox);
        let decision = permission.as_ref().map(|permission| {
            self.hands
                .permissions
                .ask(permission, self.hands.cancellation)
        });
        if let (Some(permission), Some(Decision::Deny)) = (permission, decision) {
            refused.push(permission.tool_id);
        }
        let unanswerable = self
            .stdin
            .as_mut()
            .is_some_and(|stdin| request.answer(decision, stdin).is_err());
        if unanswerable {
            *self.stdin = None;
        }
    }
}

fn read_tokens(tokens: u64) -> Option<u64> {
    (tokens > 0).then_some(tokens)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/listen/mod.rs"]
pub(crate) mod tests;
