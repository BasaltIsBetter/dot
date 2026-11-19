use std::io::{self, BufReader};
use std::path::Path;
use std::process::ChildStdin;

use dot_api::{Decision, Limits, Reply, ReplyEvent, ToolEnd, Toolbox};

use crate::claude_code::control::ControlRequest;
use crate::claude_code::launch::SessionId;
use crate::claude_code::output::{Line, Outcome, parse};
use crate::claude_code::tools::ToolResult;
use crate::line::{Read, read_line};
use crate::messages::Hands;
use crate::pricing;
use crate::stream::Turn;

const LONGEST_LINE: usize = 16 * 1_024 * 1_024;

pub(crate) struct Heard<'a> {
    pub(crate) turn: Turn<'a>,
    pub(crate) limits: Limits,
    pub(crate) outcome: Option<Outcome>,
}

pub(crate) struct Listener<'l, 'h, T> {
    pub(crate) session: &'l mut Option<SessionId>,
    pub(crate) stdin: &'l mut Option<ChildStdin>,
    pub(crate) hands: &'l Hands<'h, T>,
    pub(crate) on_event: &'l mut dyn FnMut(ReplyEvent),
}

impl<'a> Heard<'a> {
    pub(crate) fn nothing(folder: Option<&'a Path>) -> Self {
        Self {
            turn: Turn::listening(folder),
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
            cost,
            limits: self.limits,
        }
    }
}

impl<'h, T: Toolbox> Listener<'_, 'h, T> {
    pub(crate) fn listen(&mut self, stdout: impl io::Read) -> io::Result<Heard<'h>> {
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        let mut heard = Heard::nothing(self.hands.toolbox.folder());
        let mut refused = Vec::new();

        while read_line(&mut reader, &mut line, LONGEST_LINE)? != Read::End {
            match parse(&line) {
                Ok(Line::System { session_id }) => {
                    if let Some(id) = session_id.as_deref().and_then(SessionId::parse) {
                        *self.session = Some(id);
                    }
                }
                Ok(Line::Stream(event)) => heard.turn.apply(event, self.on_event),
                Ok(Line::RateLimits(limits)) => heard.limits = limits,
                Ok(Line::ToolResults(results)) => self.end_tools(results, &refused),
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
        let permission = request.permission(self.hands.toolbox);
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

    fn end_tools(&mut self, results: Vec<ToolResult>, refused: &[String]) {
        for result in results {
            let end = if refused.contains(&result.id) {
                ToolEnd::Refused
            } else if result.is_error {
                ToolEnd::Failed
            } else {
                ToolEnd::Done(result.detail)
            };
            (self.on_event)(ReplyEvent::ToolEnded { id: result.id, end });
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/listen.rs"]
mod tests;
