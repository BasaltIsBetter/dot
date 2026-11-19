use std::path::Path;

use dot_api::{ToolKind, ToolUse};
use serde_json::Value;

const SEPARATOR: char = '/';
const RESTART: &str = "restart";

pub(crate) fn shown(name: &str, input: &Value, folder: Option<&Path>) -> ToolUse {
    let field = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or_default();
    let path = |key: &str| relative(field(key), folder);
    let (kind, subject) = match name {
        "bash" if input.get(RESTART).and_then(Value::as_bool) == Some(true) => {
            (ToolKind::Run, RESTART.to_owned())
        }
        "bash" | "Bash" => (ToolKind::Run, field("command").to_owned()),
        "str_replace_based_edit_tool" => match field("command") {
            "view" => (ToolKind::Read, path("path")),
            "create" => (ToolKind::Write, path("path")),
            _ => (ToolKind::Edit, path("path")),
        },
        "Read" => (ToolKind::Read, path("file_path")),
        "Write" => (ToolKind::Write, path("file_path")),
        "Edit" | "MultiEdit" => (ToolKind::Edit, path("file_path")),
        "NotebookEdit" => (ToolKind::Edit, path("notebook_path")),
        "find_files" | "Glob" => (ToolKind::FindFiles, field("pattern").to_owned()),
        "search_text" | "Grep" => (ToolKind::SearchText, field("pattern").to_owned()),
        "web_fetch" | "WebFetch" => (ToolKind::Fetch, field("url").to_owned()),
        "web_search" | "WebSearch" => (ToolKind::SearchWeb, field("query").to_owned()),
        "Agent" | "Task" => (ToolKind::Delegate, field("description").to_owned()),
        other => (ToolKind::Other, other.to_owned()),
    };
    ToolUse { kind, subject }
}

pub(crate) fn relative(path: &str, folder: Option<&Path>) -> String {
    let inside = folder
        .and_then(Path::to_str)
        .and_then(|folder| path.strip_prefix(folder))
        .and_then(|rest| rest.strip_prefix(SEPARATOR))
        .filter(|rest| !rest.is_empty());
    inside.unwrap_or(path).to_owned()
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/tool/shown.rs"]
mod tests;
