use dot_api::{Cost, Limits, Reply, ReplyEvent, ToolContent, ToolEnd, ToolOutput, Toolbox};

use crate::messages::Hands;
use crate::pricing;
use crate::stream::{ToolUseBlock, Turn};
use crate::tool::call_of;

const THE_REPLY_ENDED_BEFORE_THE_TOOL_RAN: &str = "The reply ended before this tool ran.";
const THE_TOOL_INPUT_IS_NOT_JSON: &str = "The tool input is not a JSON object.";

#[derive(Debug, Default)]
pub(crate) struct Spent {
    micro_dollars: u64,
    context_tokens: u64,
}

impl Spent {
    pub(crate) fn add(&mut self, turn: &Turn<'_>) {
        let cost = pricing::cost(turn.model(), turn.usage()).micro_dollars();
        self.micro_dollars = self.micro_dollars.saturating_add(cost);
        self.context_tokens = turn.usage().context_tokens();
    }

    pub(crate) fn reply(&self) -> Reply {
        Reply {
            context_tokens: self.context_tokens,
            cost: Cost::from_micro_dollars(self.micro_dollars),
            limits: Limits::default(),
        }
    }
}

pub(crate) fn answer<T: Toolbox>(
    turn: &Turn<'_>,
    runs: bool,
    hands: &mut Hands<'_, T>,
    on_event: &mut dyn FnMut(ReplyEvent),
) -> Vec<(String, ToolOutput)> {
    turn.tool_uses()
        .into_iter()
        .map(|tool| {
            let output = if runs && !hands.cancellation.is_cancelled() {
                run(tool, hands)
            } else {
                not_run(hands)
            };
            on_event(ReplyEvent::ToolEnded {
                id: tool.id.to_owned(),
                end: output.end,
            });
            (tool.id.to_owned(), output)
        })
        .collect()
}

fn run<T: Toolbox>(tool: ToolUseBlock<'_>, hands: &mut Hands<'_, T>) -> ToolOutput {
    if !tool.readable {
        return ToolOutput::failed(THE_TOOL_INPUT_IS_NOT_JSON);
    }
    match call_of(tool.name, tool.input) {
        Ok(call) => hands
            .toolbox
            .run(&call, tool.id, hands.permissions, hands.cancellation),
        Err(reason) => ToolOutput::failed(reason),
    }
}

fn not_run<T>(hands: &Hands<'_, T>) -> ToolOutput {
    if hands.cancellation.is_cancelled() {
        return ToolOutput::stopped();
    }
    ToolOutput {
        content: ToolContent::Text(THE_REPLY_ENDED_BEFORE_THE_TOOL_RAN.to_owned()),
        is_error: true,
        end: ToolEnd::Stopped,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/tools.rs"]
mod tests;
