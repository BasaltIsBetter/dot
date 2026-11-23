mod agent;
mod system;

use std::borrow::Cow;

use dot_api::{Cost, Limits, ReplyFailure};
use serde::Deserialize;

use crate::claude_code::control::{ControlRequest, Request};
use crate::claude_code::tools::{ToolResult, tool_results};
use crate::stream::Event;

#[cfg(test)]
pub(crate) use self::agent::AgentCall;
pub(crate) use self::agent::{AgentLine, AgentMessage, tokens_handed_back};
pub(crate) use self::system::Compaction;

const SYSTEM: &str = "system";
const ASSISTANT: &str = "assistant";
const STREAM_EVENT: &str = "stream_event";
const RATE_LIMIT_EVENT: &str = "rate_limit_event";
const RESULT: &str = "result";
const USER: &str = "user";
const CONTROL_REQUEST: &str = "control_request";
const PERCENT: f64 = 100.0;
const MICRO_DOLLARS_PER_DOLLAR: f64 = 1_000_000.0;

#[derive(Debug)]
pub(crate) enum Line {
    System {
        session_id: Option<String>,
        compaction: Option<Compaction>,
    },
    Stream(Event),
    RateLimits(Limits),
    ToolResults {
        agent: Option<String>,
        results: Vec<ToolResult>,
    },
    Agent(AgentLine),
    Control(ControlRequest),
    Result(Outcome),
    Other,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
pub(crate) struct Outcome {
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    total_cost_usd: f64,
    api_error_status: Option<u16>,
}

#[derive(Deserialize)]
struct Kind<'a> {
    #[serde(rename = "type", borrow)]
    kind: Cow<'a, str>,
    #[serde(default, borrow)]
    parent_tool_use_id: Option<Cow<'a, str>>,
}

#[derive(Deserialize)]
struct StreamLine {
    event: Event,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
}

#[derive(Deserialize)]
struct ControlLine {
    request_id: String,
    request: Request,
}

#[derive(Deserialize)]
struct RateLimitLine {
    rate_limit_info: RateLimitInfo,
}

#[derive(Deserialize)]
struct RateLimitInfo {
    #[serde(rename = "unifiedWindows", default)]
    windows: Windows,
}

#[derive(Default, Deserialize)]
struct Windows {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
}

#[derive(Clone, Copy, Deserialize)]
struct Window {
    utilization: f64,
}

pub(crate) fn parse(line: &[u8]) -> serde_json::Result<Line> {
    let Kind {
        kind,
        parent_tool_use_id,
    } = serde_json::from_slice(line)?;
    Ok(match kind.as_ref() {
        SYSTEM => system::parse(line)?,
        ASSISTANT if parent_tool_use_id.is_some() => Line::Agent(agent::message(line)?),
        STREAM_EVENT => {
            let StreamLine {
                event,
                parent_tool_use_id,
            } = serde_json::from_slice(line)?;
            if parent_tool_use_id.is_some() {
                return Ok(Line::Other);
            }
            Line::Stream(event)
        }
        USER => {
            let (agent, results) = tool_results(serde_json::from_slice(line)?);
            Line::ToolResults { agent, results }
        }
        CONTROL_REQUEST => {
            let ControlLine {
                request_id,
                request,
            } = serde_json::from_slice(line)?;
            Line::Control(ControlRequest {
                request_id,
                request,
            })
        }
        RATE_LIMIT_EVENT => {
            let RateLimitLine { rate_limit_info } = serde_json::from_slice(line)?;
            Line::RateLimits(Limits {
                five_hours: rate_limit_info.windows.five_hour.map(percent),
                seven_days: rate_limit_info.windows.seven_day.map(percent),
            })
        }
        RESULT => Line::Result(serde_json::from_slice(line)?),
        _ => Line::Other,
    })
}

impl Outcome {
    pub(crate) fn failure(&self) -> Option<ReplyFailure> {
        if !self.is_error {
            return None;
        }
        Some(match self.api_error_status {
            Some(401 | 403) => ReplyFailure::Unauthorized,
            Some(429) => ReplyFailure::RateLimited,
            Some(503 | 529) => ReplyFailure::Overloaded,
            _ => ReplyFailure::ClaudeCodeFailed,
        })
    }

    pub(crate) fn cost(&self) -> Cost {
        Cost::from_micro_dollars(micro_dollars(self.total_cost_usd))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the share is clamped to [0, 100] before rounding, so it fits in u8"
)]
fn percent(window: Window) -> u8 {
    let share = if window.utilization.is_finite() {
        window.utilization
    } else {
        0.0
    };
    (share * PERCENT).clamp(0.0, PERCENT).round() as u8
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "negative and non-finite costs count as zero, and a float cast to u64 saturates"
)]
fn micro_dollars(dollars: f64) -> u64 {
    if !dollars.is_finite() || dollars <= 0.0 {
        return 0;
    }
    (dollars * MICRO_DOLLARS_PER_DOLLAR).round() as u64
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/output/mod.rs"]
mod tests;
