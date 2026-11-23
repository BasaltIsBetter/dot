use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub(crate) enum AgentLine {
    Started { tool: String, kind: String },
    Reported { tool: String, text: String },
    Message(AgentMessage),
}

#[derive(Debug, PartialEq)]
pub(crate) struct AgentMessage {
    pub(crate) tool: String,
    pub(crate) said: Vec<String>,
    pub(crate) calls: Vec<AgentCall>,
    pub(crate) tokens: u64,
}

#[derive(Debug, PartialEq)]
pub(crate) struct AgentCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input: Value,
}

#[derive(Deserialize)]
struct MessageLine {
    message: Message,
    parent_tool_use_id: String,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Vec<Block>,
    #[serde(default)]
    usage: Written,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Default, Deserialize)]
struct Written {
    #[serde(default)]
    output_tokens: u64,
}

pub(crate) fn message(line: &[u8]) -> serde_json::Result<AgentLine> {
    let MessageLine {
        message,
        parent_tool_use_id,
    } = serde_json::from_slice(line)?;
    let mut said = Vec::new();
    let mut calls = Vec::new();
    for block in message.content {
        match block {
            Block::Text { text } if !text.trim().is_empty() => said.push(text),
            Block::ToolUse { id, name, input } => calls.push(AgentCall { id, name, input }),
            Block::Text { .. } | Block::Other => {}
        }
    }
    Ok(AgentLine::Message(AgentMessage {
        tool: parent_tool_use_id,
        said,
        calls,
        tokens: message.usage.output_tokens,
    }))
}

pub(crate) fn tokens_handed_back(result: &Value) -> Option<u64> {
    result.get("agentType")?;
    result
        .get("usage")
        .and_then(|usage| usage.get("output_tokens"))
        .and_then(Value::as_u64)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/output/agent.rs"]
mod tests;
