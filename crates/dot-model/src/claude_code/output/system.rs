use std::borrow::Cow;

use serde::Deserialize;

use crate::claude_code::output::{AgentLine, Line};

const COMPACT_BOUNDARY: &str = "compact_boundary";
const TASK_STARTED: &str = "task_started";
const TASK_NOTIFICATION: &str = "task_notification";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Compaction {
    #[serde(rename = "pre_tokens")]
    pub(crate) before: u64,
    #[serde(rename = "post_tokens")]
    pub(crate) after: u64,
}

#[derive(Deserialize)]
struct SystemLine<'a> {
    session_id: Option<String>,
    #[serde(borrow)]
    subtype: Option<Cow<'a, str>>,
    compact_metadata: Option<Compaction>,
    tool_use_id: Option<String>,
    subagent_type: Option<String>,
    summary: Option<String>,
}

pub(super) fn parse(line: &[u8]) -> serde_json::Result<Line> {
    let SystemLine {
        session_id,
        subtype,
        compact_metadata,
        tool_use_id,
        subagent_type,
        summary,
    } = serde_json::from_slice(line)?;
    let about_an_agent = match (subtype.as_deref(), tool_use_id) {
        (Some(TASK_STARTED), Some(tool)) => Some(AgentLine::Started {
            tool,
            kind: subagent_type.unwrap_or_default(),
        }),
        (Some(TASK_NOTIFICATION), Some(tool)) => Some(AgentLine::Reported {
            tool,
            text: summary.unwrap_or_default(),
        }),
        _ => None,
    };
    if let Some(agent) = about_an_agent {
        return Ok(Line::Agent(agent));
    }
    let compaction = compact_metadata.filter(|_| subtype.as_deref() == Some(COMPACT_BOUNDARY));
    Ok(Line::System {
        session_id,
        compaction,
    })
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/output/system.rs"]
mod tests;
