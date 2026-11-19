use std::path::Path;

use dot_api::{ToolKind, ToolUse};
use serde_json::Value;

const SEPARATOR: char = '/';
const RESTART: &str = "restart";

pub(crate) fn described<'a>(name: &'a str, input: &'a Value) -> (ToolKind, &'a str) {
    let field = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or_default();
    match name {
        "bash" if input.get(RESTART).and_then(Value::as_bool) == Some(true) => {
            (ToolKind::Run, RESTART)
        }
        "bash" | "Bash" => (ToolKind::Run, field("command")),
        "str_replace_based_edit_tool" => match field("command") {
            "view" => (ToolKind::Read, field("path")),
            "create" => (ToolKind::Write, field("path")),
            _ => (ToolKind::Edit, field("path")),
        },
        "Read" => (ToolKind::Read, field("file_path")),
        "Write" => (ToolKind::Write, field("file_path")),
        "Edit" | "MultiEdit" => (ToolKind::Edit, field("file_path")),
        "NotebookEdit" => (ToolKind::Edit, field("notebook_path")),
        "find_files" | "Glob" => (ToolKind::FindFiles, field("pattern")),
        "search_text" | "Grep" => (ToolKind::SearchText, field("pattern")),
        "web_fetch" | "WebFetch" => (ToolKind::Fetch, field("url")),
        "web_search" | "WebSearch" => (ToolKind::SearchWeb, field("query")),
        "Agent" | "Task" => (ToolKind::Delegate, field("description")),
        other => (ToolKind::Other, other),
    }
}

pub(crate) fn shown(name: &str, input: &Value, folder: Option<&Path>) -> ToolUse {
    let (kind, subject) = described(name, input);
    let subject = match kind {
        ToolKind::Read | ToolKind::Write | ToolKind::Edit => relative(subject, folder),
        _ => subject.to_owned(),
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
