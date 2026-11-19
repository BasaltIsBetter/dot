use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const TYPE: &str = "type";
const FALLBACK: &str = "fallback";

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Known {
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

impl Known {
    pub(crate) fn read(block: &Map<String, Value>) -> Option<Self> {
        Self::deserialize(block).ok()
    }
}

pub(crate) fn is_fallback(block: &Map<String, Value>) -> bool {
    block.get(TYPE).and_then(Value::as_str) == Some(FALLBACK)
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
