use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const TYPE: &str = "type";
const KNOWN_TYPES: [&str; 5] = [
    "text",
    "thinking",
    "redacted_thinking",
    "tool_use",
    "server_tool_use",
];
const FALLBACK: &str = "fallback";

#[derive(Debug, PartialEq)]
pub(crate) enum StartedBlock {
    Text {
        text: String,
        citations: Vec<Value>,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    ToolUse {
        id: String,
        name: String,
        server: bool,
    },
    Fallback(Map<String, Value>),
    Raw(Map<String, Value>),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Known {
    Text {
        #[serde(default)]
        text: String,
        #[serde(default)]
        citations: Option<Vec<Value>>,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default)]
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    ToolUse {
        id: String,
        name: String,
    },
    ServerToolUse {
        id: String,
        name: String,
    },
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum Replayed<'a> {
    Typed(Typed<'a>),
    Raw(&'a Map<String, Value>),
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Typed<'a> {
    Text {
        text: &'a str,
        #[serde(skip_serializing_if = "<[Value]>::is_empty")]
        citations: &'a [Value],
    },
    Thinking {
        thinking: &'a str,
        signature: &'a str,
    },
    RedactedThinking {
        data: &'a str,
    },
    ToolUse {
        id: &'a str,
        name: &'a str,
        input: &'a Value,
    },
    ServerToolUse {
        id: &'a str,
        name: &'a str,
        input: &'a Value,
    },
}

impl StartedBlock {
    pub(crate) fn of(block: Map<String, Value>) -> Self {
        let kind = block.get(TYPE).and_then(Value::as_str);
        if kind == Some(FALLBACK) {
            return Self::Fallback(block);
        }
        if !kind.is_some_and(|kind| KNOWN_TYPES.contains(&kind)) {
            return Self::Raw(block);
        }
        let Ok(known) = Known::deserialize(&Value::Object(block.clone())) else {
            return Self::Raw(block);
        };
        match known {
            Known::Text { text, citations } => Self::Text {
                text,
                citations: citations.unwrap_or_default(),
            },
            Known::Thinking {
                thinking,
                signature,
            } => Self::Thinking {
                thinking,
                signature,
            },
            Known::RedactedThinking { data } => Self::RedactedThinking { data },
            Known::ToolUse { id, name } => Self::ToolUse {
                id,
                name,
                server: false,
            },
            Known::ServerToolUse { id, name } => Self::ToolUse {
                id,
                name,
                server: true,
            },
        }
    }
}

impl<'a> Typed<'a> {
    pub(crate) fn tool_use(id: &'a str, name: &'a str, input: &'a Value, server: bool) -> Self {
        if server {
            Self::ServerToolUse { id, name, input }
        } else {
            Self::ToolUse { id, name, input }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/stream/block.rs"]
mod tests;
