pub(crate) mod block;
mod content;
mod turn;

use dot_api::ReplyFailure;
use serde::Deserialize;
use serde_json::{Map, Value};

pub(crate) use self::block::Replayed;
pub(crate) use self::turn::{ToolUseBlock, Turn};

const OVERLOADED: &str = "overloaded_error";
const RATE_LIMITED: &str = "rate_limit_error";
const UNAUTHORIZED: [&str; 2] = ["authentication_error", "permission_error"];
const REJECTED: [&str; 3] = [
    "invalid_request_error",
    "not_found_error",
    "request_too_large",
];

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Event {
    MessageStart {
        message: Message,
    },
    ContentBlockStart {
        content_block: Map<String, Value>,
    },
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
        #[serde(default)]
        usage: Usage,
    },
    MessageStop,
    Error {
        error: ApiError,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Message {
    #[serde(default)]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Delta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    #[serde(rename = "signature_delta")]
    Signature { signature: String },
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    #[serde(rename = "citations_delta")]
    Citation { citation: Value },
    #[serde(other)]
    Other,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct MessageDelta {
    pub(crate) stop_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub(crate) struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation: Option<CacheCreation>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
struct CacheCreation {
    ephemeral_1h_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize, thiserror::Error)]
#[error("Anthropic answered with {kind}: {message}")]
pub(crate) struct ApiError {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    message: String,
}

impl Usage {
    pub(crate) fn merge(&mut self, newer: Self) {
        self.input_tokens = newer.input_tokens.or(self.input_tokens);
        self.output_tokens = newer.output_tokens.or(self.output_tokens);
        self.cache_creation_input_tokens = newer
            .cache_creation_input_tokens
            .or(self.cache_creation_input_tokens);
        self.cache_read_input_tokens = newer
            .cache_read_input_tokens
            .or(self.cache_read_input_tokens);
        self.cache_creation = newer.cache_creation.or(self.cache_creation);
    }

    pub(crate) fn context_tokens(&self) -> u64 {
        self.prompt_tokens().saturating_add(self.output())
    }

    pub(crate) fn prompt_tokens(&self) -> u64 {
        self.input()
            .saturating_add(self.cache_read())
            .saturating_add(self.cache_written())
    }

    pub(crate) fn input(&self) -> u64 {
        self.input_tokens.unwrap_or_default()
    }

    pub(crate) fn output(&self) -> u64 {
        self.output_tokens.unwrap_or_default()
    }

    pub(crate) fn cache_read(&self) -> u64 {
        self.cache_read_input_tokens.unwrap_or_default()
    }

    pub(crate) fn cache_written(&self) -> u64 {
        self.cache_creation_input_tokens.unwrap_or_default()
    }

    pub(crate) fn cache_written_for_an_hour(&self) -> u64 {
        self.cache_creation
            .and_then(|creation| creation.ephemeral_1h_input_tokens)
            .unwrap_or_default()
            .min(self.cache_written())
    }
}

impl ApiError {
    pub(crate) fn failure(&self) -> ReplyFailure {
        failure_of_kind(&self.kind)
    }
}

pub(crate) fn failure_of_kind(kind: &str) -> ReplyFailure {
    match kind {
        OVERLOADED => ReplyFailure::Overloaded,
        RATE_LIMITED => ReplyFailure::RateLimited,
        unauthorized if UNAUTHORIZED.contains(&unauthorized) => ReplyFailure::Unauthorized,
        rejected if REJECTED.contains(&rejected) => ReplyFailure::Rejected,
        _ => ReplyFailure::Unreachable,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/stream/mod.rs"]
mod tests;
