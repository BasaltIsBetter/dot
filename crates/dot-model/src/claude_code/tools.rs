use dot_api::Detail;
use serde::Deserialize;
use serde_json::Value;

const TOOL_RESULT: &str = "tool_result";
const ADDED: char = '+';
const REMOVED: char = '-';
const NEW_LINE: char = '\n';

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ToolResult {
    pub(crate) id: String,
    pub(crate) is_error: bool,
    pub(crate) detail: Detail,
}

#[derive(Deserialize)]
pub(crate) struct UserLine {
    message: UserContent,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
    #[serde(default)]
    tool_use_result: Value,
}

#[derive(Deserialize)]
struct UserContent {
    #[serde(default)]
    content: Value,
}

#[derive(Deserialize)]
struct ResultBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    tool_use_id: String,
    #[serde(default)]
    is_error: bool,
}

pub(crate) fn tool_results(line: UserLine) -> Vec<ToolResult> {
    if line.parent_tool_use_id.is_some() {
        return Vec::new();
    }
    let Value::Array(blocks) = line.message.content else {
        return Vec::new();
    };
    let detail = detail_of(&line.tool_use_result);
    blocks
        .into_iter()
        .filter_map(|block| ResultBlock::deserialize(block).ok())
        .filter(|block| block.kind == TOOL_RESULT)
        .map(|block| ToolResult {
            id: block.tool_use_id,
            is_error: block.is_error,
            detail,
        })
        .collect()
}

fn detail_of(result: &Value) -> Detail {
    let number = |key: &str| result.get(key).and_then(Value::as_u64);
    if let Some(Value::Array(hunks)) = result.get("structuredPatch") {
        let (added, removed) = changes(hunks);
        let created = result.get("content").and_then(Value::as_str);
        if added == 0
            && removed == 0
            && let Some(created) = created
        {
            return Detail::Changes {
                added: line_count(created),
                removed: 0,
            };
        }
        return Detail::Changes { added, removed };
    }
    if let Some(lines) = result.get("file").and_then(|file| file.get("numLines")) {
        return lines.as_u64().map_or(Detail::Nothing, Detail::Lines);
    }
    if let Some(lines) = number("numLines").filter(|_| result.get("content").is_some()) {
        return Detail::Matches(lines);
    }
    number("numFiles").map_or(Detail::Nothing, Detail::Files)
}

fn changes(hunks: &[Value]) -> (u64, u64) {
    let lines = hunks
        .iter()
        .filter_map(|hunk| hunk.get("lines").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str);
    lines.fold((0, 0), |(added, removed), line| {
        if line.starts_with(ADDED) {
            (added + 1, removed)
        } else if line.starts_with(REMOVED) {
            (added, removed + 1)
        } else {
            (added, removed)
        }
    })
}

fn line_count(text: &str) -> u64 {
    let breaks = text.matches(NEW_LINE).count();
    let unfinished = usize::from(!text.is_empty() && !text.ends_with(NEW_LINE));
    u64::try_from(breaks + unfinished).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/tools.rs"]
mod tests;
