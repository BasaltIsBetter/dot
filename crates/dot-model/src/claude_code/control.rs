use std::io::{self, Write};

use dot_api::{Action, Decision, Permission, Toolbox, USER_REFUSED_THE_TOOL};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::relative;

const CAN_USE_TOOL: &str = "can_use_tool";
const CONTROL_RESPONSE: &str = "control_response";
const DOT_DOES_NOT_HANDLE_THIS_REQUEST: &str = "Dot does not handle this request.";
const NEW_LINE: &[u8] = b"\n";

#[derive(Debug)]
pub(crate) struct ControlRequest {
    pub(crate) request_id: String,
    pub(crate) request: Request,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    subtype: String,
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    input: Value,
    #[serde(default)]
    tool_use_id: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Serialize)]
struct ControlResponse<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    response: Body<'a>,
}

#[derive(Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
enum Body<'a> {
    Success {
        request_id: &'a str,
        response: Verdict<'a>,
    },
    Error {
        request_id: &'a str,
        error: &'static str,
    },
}

#[derive(Serialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
enum Verdict<'a> {
    Allow {
        #[serde(rename = "updatedInput")]
        updated_input: &'a Value,
    },
    Deny {
        message: &'static str,
        interrupt: bool,
    },
}

impl ControlRequest {
    pub(crate) fn permission(&self, toolbox: &impl Toolbox) -> Option<Permission> {
        let request = &self.request;
        if request.subtype != CAN_USE_TOOL {
            return None;
        }
        Some(Permission {
            tool_id: request.tool_use_id.clone(),
            action: request.action(toolbox),
        })
    }

    pub(crate) fn answer(&self, decision: Option<Decision>, to: &mut impl Write) -> io::Result<()> {
        let request_id = self.request_id.as_str();
        let response = match decision {
            Some(Decision::Allow) => Body::Success {
                request_id,
                response: Verdict::Allow {
                    updated_input: &self.request.input,
                },
            },
            Some(Decision::Deny) => Body::Success {
                request_id,
                response: Verdict::Deny {
                    message: USER_REFUSED_THE_TOOL,
                    interrupt: false,
                },
            },
            None => Body::Error {
                request_id,
                error: DOT_DOES_NOT_HANDLE_THIS_REQUEST,
            },
        };
        let mut line = serde_json::to_vec(&ControlResponse {
            kind: CONTROL_RESPONSE,
            response,
        })?;
        line.extend_from_slice(NEW_LINE);
        to.write_all(&line)?;
        to.flush()
    }
}

impl Request {
    fn action(&self, toolbox: &impl Toolbox) -> Action {
        let field = |key: &str| {
            self.input
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        match self.tool_name.as_str() {
            "Bash" => Action::Run {
                command: field("command"),
            },
            "Write" | "Edit" | "MultiEdit" => self.change(field("file_path"), toolbox),
            "NotebookEdit" => self.change(field("notebook_path"), toolbox),
            "Read" => Action::ReadOutside {
                path: field("file_path"),
            },
            "Glob" | "Grep" => Action::ReadOutside {
                path: field("path"),
            },
            "WebFetch" => Action::Fetch { url: field("url") },
            "WebSearch" => Action::SearchWeb {
                query: field("query"),
            },
            tool => Action::Use {
                tool: tool.to_owned(),
                detail: self.description.clone().unwrap_or_default(),
            },
        }
    }

    fn change(&self, path: String, toolbox: &impl Toolbox) -> Action {
        if path.is_empty() {
            return Action::Use {
                tool: self.tool_name.clone(),
                detail: self.description.clone().unwrap_or_default(),
            };
        }
        if toolbox.holds(&path) {
            return Action::Change {
                path: relative(&path, toolbox.folder()),
            };
        }
        Action::ChangeOutside { path }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/control.rs"]
mod tests;
