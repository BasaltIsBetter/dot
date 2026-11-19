use std::path::Path;

use dot_api::ReplyEvent;
use serde_json::Value;

use crate::stream::block::{Replayed, StartedBlock};
use crate::stream::content::{Block, Content};
use crate::stream::{Delta, Event, Usage};
use crate::tool::shown;

const REFUSAL: &str = "refusal";
const TOOL_USE: &str = "tool_use";
const PAUSE_TURN: &str = "pause_turn";

#[derive(Debug, Default)]
pub(crate) struct Turn<'a> {
    keeps_content: bool,
    folder: Option<&'a Path>,
    blocks: Vec<Block>,
    first_of_message: usize,
    model: String,
    usage: Usage,
    stop_reason: Option<String>,
    finished: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ToolUseBlock<'t> {
    pub(crate) id: &'t str,
    pub(crate) name: &'t str,
    pub(crate) input: &'t Value,
    pub(crate) readable: bool,
}

impl<'a> Turn<'a> {
    pub(crate) fn replayable(folder: Option<&'a Path>) -> Self {
        Self {
            keeps_content: true,
            folder,
            ..Self::default()
        }
    }

    pub(crate) fn listening(folder: Option<&'a Path>) -> Self {
        Self {
            folder,
            ..Self::default()
        }
    }

    pub(crate) fn apply(&mut self, event: Event, on_event: &mut dyn FnMut(ReplyEvent)) {
        match event {
            Event::MessageStart { message } => {
                self.first_of_message = self.blocks.len();
                self.model = message.model;
                self.usage = message.usage;
                self.finished = false;
            }
            Event::ContentBlockStart { content_block } => {
                let content = Content::of(
                    StartedBlock::of(content_block),
                    self.keeps_content,
                    on_event,
                );
                self.blocks.push(Block {
                    content,
                    complete: false,
                });
            }
            Event::ContentBlockDelta { index, delta } => self.extend(index, delta, on_event),
            Event::ContentBlockStop { index } => self.stop(index, on_event),
            Event::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason.or(self.stop_reason.take());
                self.usage.merge(usage);
            }
            Event::MessageStop => self.finished = true,
            Event::Error { .. } | Event::Other => {}
        }
    }

    pub(crate) const fn is_finished(&self) -> bool {
        self.finished
    }

    pub(crate) fn is_declined(&self) -> bool {
        self.stop_reason.as_deref() == Some(REFUSAL)
    }

    pub(crate) fn wants_tools(&self) -> bool {
        self.stop_reason.as_deref() == Some(TOOL_USE)
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.stop_reason.as_deref() == Some(PAUSE_TURN)
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }

    pub(crate) const fn usage(&self) -> &Usage {
        &self.usage
    }

    pub(crate) fn tool_uses(&self) -> Vec<ToolUseBlock<'_>> {
        self.blocks
            .iter()
            .filter(|block| block.complete)
            .filter_map(|block| match &block.content {
                Content::ToolUse(tool) if !tool.server => Some(ToolUseBlock {
                    id: &tool.id,
                    name: &tool.name,
                    input: &tool.input,
                    readable: tool.readable,
                }),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn replayed(&self) -> Vec<Replayed<'_>> {
        let last_fallback = self
            .blocks
            .iter()
            .rposition(|block| matches!(block.content, Content::Fallback(_)));
        let last = self.blocks.len().saturating_sub(1);

        self.blocks
            .iter()
            .enumerate()
            .filter(|&(position, block)| block.complete || position == last)
            .filter_map(|(position, block)| {
                block.replayed(last_fallback.is_some_and(|fallback| position < fallback))
            })
            .collect()
    }

    fn extend(&mut self, index: usize, delta: Delta, on_event: &mut dyn FnMut(ReplyEvent)) {
        let content = self
            .blocks
            .get_mut(self.first_of_message + index)
            .map(|block| &mut block.content);
        match (delta, content) {
            (Delta::Text { text }, content) => {
                if let Some(Content::Text { text: kept, .. }) = content {
                    kept.push_str(&text);
                }
                on_event(ReplyEvent::Text(text));
            }
            (Delta::Thinking { thinking }, content) => {
                if let Some(Content::Thinking { thinking: kept, .. }) = content {
                    kept.push_str(&thinking);
                }
                if !thinking.is_empty() {
                    on_event(ReplyEvent::Thinking(thinking));
                }
            }
            (
                Delta::Signature { signature },
                Some(Content::Thinking {
                    signature: kept, ..
                }),
            ) => kept.push_str(&signature),
            (Delta::InputJson { partial_json }, Some(Content::ToolUse(tool))) => {
                tool.written.push_str(&partial_json);
            }
            (Delta::Citation { citation }, Some(Content::Text { citations, .. })) => {
                citations.push(citation);
            }
            (
                Delta::Signature { .. }
                | Delta::InputJson { .. }
                | Delta::Citation { .. }
                | Delta::Other,
                _,
            ) => {}
        }
    }

    fn stop(&mut self, index: usize, on_event: &mut dyn FnMut(ReplyEvent)) {
        let folder = self.folder;
        let Some(block) = self.blocks.get_mut(self.first_of_message + index) else {
            return;
        };
        block.complete = true;
        let Content::ToolUse(tool) = &mut block.content else {
            return;
        };
        tool.finish();
        on_event(ReplyEvent::ToolStarted {
            id: tool.id.clone(),
            tool: shown(&tool.name, &tool.input, folder),
        });
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/stream/turn.rs"]
mod tests;
