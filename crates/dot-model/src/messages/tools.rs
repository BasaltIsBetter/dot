use dot_api::{
    Compactor, Cost, Detail, Limits, Origin, Reply, ReplyEvent, SearchShow, ToolCall, ToolContent,
    ToolEnd, ToolOutput, Toolbox,
};

use crate::change::as_shown;
use crate::live::LiveTail;
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
    output_tokens: u64,
    starts_a_conversation: bool,
    base_tokens: Option<u64>,
}

impl Spent {
    pub(crate) fn of_a_reply(starts_a_conversation: bool) -> Self {
        Self {
            starts_a_conversation,
            ..Self::default()
        }
    }

    pub(crate) fn add(&mut self, turn: &Turn<'_>) {
        let cost = pricing::cost(turn.model(), turn.usage()).micro_dollars();
        self.micro_dollars = self.micro_dollars.saturating_add(cost);
        self.context_tokens = turn.usage().context_tokens();
        self.output_tokens = self.output_tokens.saturating_add(turn.output_tokens());

        let read = turn.usage().prompt_tokens();
        if self.starts_a_conversation && self.base_tokens.is_none() && read > 0 {
            self.base_tokens = Some(read);
        }
    }

    pub(crate) fn reply(&self) -> Reply {
        Reply {
            context_tokens: self.context_tokens,
            base_tokens: self.base_tokens,
            output_tokens: self.output_tokens,
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
            let mut output = if runs && !hands.cancellation.is_cancelled() {
                run(tool, hands, on_event)
            } else {
                not_run(hands)
            };
            if let Some(change) = output.change.take() {
                let change = as_shown(*change, hands.toolbox.folder());
                on_event(ReplyEvent::ToolChanged {
                    id: tool.id.to_owned(),
                    change: Box::new(change),
                });
            }
            on_event(ReplyEvent::ToolEnded {
                id: tool.id.to_owned(),
                end: output.end,
            });
            (tool.id.to_owned(), output)
        })
        .collect()
}

fn run<T: Toolbox>(
    tool: ToolUseBlock<'_>,
    hands: &mut Hands<'_, T>,
    on_event: &mut dyn FnMut(ReplyEvent),
) -> ToolOutput {
    if !tool.readable {
        return ToolOutput::failed(THE_TOOL_INPUT_IS_NOT_JSON);
    }
    let call = match call_of(tool.name, tool.input) {
        Ok(call) => call,
        Err(reason) => return ToolOutput::failed(reason),
    };

    let compactor = hands.compactor;
    let mut tail = LiveTail::default();
    let mut on_output = |piece: &[u8]| {
        tail.push(piece);
        on_event(ReplyEvent::ToolOutput {
            id: tool.id.to_owned(),
            lines: tail.lines(compactor),
        });
    };
    let output = hands.toolbox.run(
        &call,
        tool.id,
        hands.permissions,
        hands.cancellation,
        &mut on_output,
    );
    shortened(&call, output, compactor)
}

fn shortened(call: &ToolCall, mut output: ToolOutput, compactor: &dyn Compactor) -> ToolOutput {
    let Some(origin) = origin_of(call, &output) else {
        return output;
    };
    if let ToolContent::Text(text) = &output.content
        && let Some(shorter) = compactor.compact(origin, text)
    {
        tracing::info!(
            before = text.len(),
            after = shorter.len(),
            "tool output shortened"
        );
        output.content = ToolContent::Text(shorter);
    }
    output
}

fn origin_of<'a>(call: &'a ToolCall, output: &ToolOutput) -> Option<Origin<'a>> {
    match call {
        ToolCall::Run { command } if matches!(output.end, ToolEnd::Done(Detail::Exit(_))) => {
            Some(Origin::Command(command))
        }
        ToolCall::FindFiles { .. } if !output.is_error => Some(Origin::Paths),
        ToolCall::SearchText(search) if !output.is_error => match search.show {
            SearchShow::Lines => Some(Origin::Matches),
            SearchShow::Files => Some(Origin::Paths),
            SearchShow::Counts => None,
        },
        _ => None,
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
        change: None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/tools.rs"]
mod tests;
