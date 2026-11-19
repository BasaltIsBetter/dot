use dot_api::{Detail, ReplyEvent, ToolEnd};
use serde_json::{Map, Value};

use crate::stream::block::{Known, Replayed, Typed, is_fallback};

const TOOL_USE_ID: &str = "tool_use_id";
const CONTENT: &str = "content";
const ERROR_CODE: &str = "error_code";
const RETURN_CODE: &str = "return_code";

#[derive(Debug)]
pub(crate) struct Block {
    pub(crate) content: Content,
    pub(crate) complete: bool,
}

#[derive(Debug)]
pub(crate) enum Content {
    Text { text: String, citations: Vec<Value> },
    Thinking { thinking: String, signature: String },
    RedactedThinking(String),
    Fallback(Map<String, Value>),
    ToolUse(ToolUse),
    Raw(Map<String, Value>),
    Other,
}

#[derive(Debug)]
pub(crate) struct ToolUse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) server: bool,
    pub(crate) written: String,
    pub(crate) input: Value,
    pub(crate) readable: bool,
}

impl Content {
    pub(crate) fn of(
        block: Map<String, Value>,
        keeps_content: bool,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Self {
        let content = match Known::read(&block) {
            Some(Known::Text { text, citations }) => {
                if !text.is_empty() {
                    on_event(ReplyEvent::Text(text.clone()));
                }
                Self::Text {
                    text,
                    citations: citations.unwrap_or_default(),
                }
            }
            Some(Known::Thinking {
                thinking,
                signature,
            }) => {
                if !thinking.is_empty() {
                    on_event(ReplyEvent::Thinking(thinking.clone()));
                }
                Self::Thinking {
                    thinking,
                    signature,
                }
            }
            Some(Known::RedactedThinking { data }) => Self::RedactedThinking(data),
            Some(Known::ToolUse { id, name }) => {
                return Self::ToolUse(ToolUse::new(id, name, false));
            }
            Some(Known::ServerToolUse { id, name }) => {
                return Self::ToolUse(ToolUse::new(id, name, true));
            }
            None if is_fallback(&block) => Self::Fallback(block),
            None => {
                if let Some(id) = block.get(TOOL_USE_ID).and_then(Value::as_str) {
                    on_event(ReplyEvent::ToolEnded {
                        id: id.to_owned(),
                        end: server_end(&block),
                    });
                }
                Self::Raw(block)
            }
        };
        if keeps_content { content } else { Self::Other }
    }
}

impl Block {
    pub(crate) fn replayed(&self, before_fallback: bool) -> Option<Replayed<'_>> {
        let typed = match &self.content {
            Content::Text { text, citations } if !text.is_empty() => {
                Typed::Text { text, citations }
            }
            Content::Thinking {
                thinking,
                signature,
            } if self.complete && !before_fallback && !signature.is_empty() => Typed::Thinking {
                thinking,
                signature,
            },
            Content::RedactedThinking(data) if !before_fallback => Typed::RedactedThinking { data },
            Content::ToolUse(tool) if self.complete => {
                Typed::tool_use(&tool.id, &tool.name, &tool.input, tool.server)
            }
            Content::Fallback(fields) | Content::Raw(fields) => return Some(Replayed::Raw(fields)),
            _ => return None,
        };
        Some(Replayed::Typed(typed))
    }
}

impl ToolUse {
    fn new(id: String, name: String, server: bool) -> Self {
        Self {
            id,
            name,
            server,
            written: String::new(),
            input: Value::Object(Map::new()),
            readable: true,
        }
    }

    pub(crate) fn finish(&mut self) {
        if self.written.trim().is_empty() {
            return;
        }
        match serde_json::from_str::<Value>(&self.written) {
            Ok(input @ Value::Object(_)) => self.input = input,
            Ok(_) | Err(_) => self.readable = false,
        }
    }
}

fn server_end(result: &Map<String, Value>) -> ToolEnd {
    match result.get(CONTENT) {
        Some(Value::Array(found)) => ToolEnd::Done(Detail::Matches(count(found.len()))),
        Some(Value::Object(fields)) if fields.contains_key(ERROR_CODE) => ToolEnd::Failed,
        Some(Value::Object(fields)) => fields
            .get(RETURN_CODE)
            .and_then(Value::as_i64)
            .and_then(|code| i32::try_from(code).ok())
            .map_or(ToolEnd::Done(Detail::Nothing), |code| {
                ToolEnd::Done(Detail::Exit(code))
            }),
        _ => ToolEnd::Done(Detail::Nothing),
    }
}

fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/stream/content.rs"]
mod tests;
