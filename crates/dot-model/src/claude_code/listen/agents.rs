use std::path::Path;

use dot_api::{AgentEvent, ReplyEvent, ToolEnd};

use crate::change::as_shown;
use crate::claude_code::output::{AgentLine, AgentMessage};
use crate::claude_code::tools::ToolResult;
use crate::tool::shown;

#[derive(Debug, Default)]
pub(crate) struct Agents {
    reports: Vec<(String, String)>,
}

impl Agents {
    pub(crate) fn hear(
        &mut self,
        line: AgentLine,
        folder: Option<&Path>,
        emit: &mut dyn FnMut(ReplyEvent),
    ) {
        match line {
            AgentLine::Started { tool, kind } => emit(agent(tool, AgentEvent::Started { kind })),
            AgentLine::Reported { tool, text } => self.reports.push((tool, text)),
            AgentLine::Message(message) => said_and_called(message, folder, emit),
        }
    }

    pub(crate) fn handed_back(&mut self, tool: &str, tokens: u64) -> ReplyEvent {
        let text = self
            .reports
            .iter()
            .position(|(reported, _)| reported == tool)
            .map_or_else(String::new, |at| self.reports.swap_remove(at).1);
        agent(tool.to_owned(), AgentEvent::Reported { text, tokens })
    }
}

fn said_and_called(message: AgentMessage, folder: Option<&Path>, emit: &mut dyn FnMut(ReplyEvent)) {
    let AgentMessage {
        tool,
        said,
        calls,
        tokens,
    } = message;
    if tokens > 0 {
        emit(agent(tool.clone(), AgentEvent::Wrote { tokens }));
    }
    for text in said {
        emit(agent(tool.clone(), AgentEvent::Said(text)));
    }
    for call in calls {
        let started = AgentEvent::ToolStarted {
            tool: shown(&call.name, &call.input, folder),
            id: call.id,
        };
        emit(agent(tool.clone(), started));
    }
}

pub(crate) fn end_tools(
    results: Vec<ToolResult>,
    (agent_of, refused, folder): (Option<String>, &[String], Option<&Path>),
    agents: &mut Agents,
    emit: &mut dyn FnMut(ReplyEvent),
) {
    for result in results {
        let end = if refused.contains(&result.id) {
            ToolEnd::Refused
        } else if result.is_error {
            ToolEnd::Failed
        } else {
            ToolEnd::Done(result.detail)
        };
        let id = result.id;
        if let Some(parent) = &agent_of {
            emit(agent(parent.clone(), AgentEvent::ToolEnded { id, end }));
            continue;
        }
        if let (ToolEnd::Done(_), Some(change)) = (&end, result.change) {
            let change = as_shown(change, folder);
            emit(ReplyEvent::ToolChanged {
                id: id.clone(),
                change: Box::new(change),
            });
        }
        if let Some(tokens) = result.agent_tokens {
            emit(agents.handed_back(&id, tokens));
        }
        emit(ReplyEvent::ToolEnded { id, end });
    }
}

fn agent(id: String, event: AgentEvent) -> ReplyEvent {
    ReplyEvent::Agent { id, event }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/listen/agents.rs"]
mod tests;
