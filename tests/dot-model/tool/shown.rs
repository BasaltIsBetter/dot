use serde_json::json;

use super::*;

fn show(name: &str, input: &Value) -> (ToolKind, String) {
    let ToolUse { kind, subject } = shown(name, input, Some(Path::new("/project")));
    (kind, subject)
}

#[test]
fn shows_the_messages_api_tools() {
    let tools = [
        (
            "bash",
            json!({"command": "cargo test"}),
            ToolKind::Run,
            "cargo test",
        ),
        ("bash", json!({"restart": true}), ToolKind::Run, "restart"),
        (
            "bash",
            json!({"restart": false, "command": "ls"}),
            ToolKind::Run,
            "ls",
        ),
        (
            "str_replace_based_edit_tool",
            json!({"command": "view", "path": "/project/src/a.rs"}),
            ToolKind::Read,
            "src/a.rs",
        ),
        (
            "str_replace_based_edit_tool",
            json!({"command": "create", "path": "/elsewhere/b.rs"}),
            ToolKind::Write,
            "/elsewhere/b.rs",
        ),
        (
            "str_replace_based_edit_tool",
            json!({"command": "str_replace", "path": "/project/c.rs"}),
            ToolKind::Edit,
            "c.rs",
        ),
        (
            "find_files",
            json!({"pattern": "**/*.rs"}),
            ToolKind::FindFiles,
            "**/*.rs",
        ),
        (
            "search_text",
            json!({"pattern": "fn main"}),
            ToolKind::SearchText,
            "fn main",
        ),
        (
            "web_fetch",
            json!({"url": "https://docs.rs"}),
            ToolKind::Fetch,
            "https://docs.rs",
        ),
        (
            "web_search",
            json!({"query": "ureq"}),
            ToolKind::SearchWeb,
            "ureq",
        ),
    ];

    for (name, input, kind, subject) in tools {
        assert_eq!(
            show(name, &input),
            (kind, subject.to_owned()),
            "{name} {input}"
        );
    }
}

#[test]
fn shows_claude_code_s_tools() {
    let tools = [
        ("Bash", json!({"command": "ls"}), ToolKind::Run, "ls"),
        (
            "Read",
            json!({"file_path": "/project/a.rs"}),
            ToolKind::Read,
            "a.rs",
        ),
        (
            "Write",
            json!({"file_path": "/project/b.rs"}),
            ToolKind::Write,
            "b.rs",
        ),
        (
            "Edit",
            json!({"file_path": "/project/c.rs"}),
            ToolKind::Edit,
            "c.rs",
        ),
        (
            "MultiEdit",
            json!({"file_path": "/project/d.rs"}),
            ToolKind::Edit,
            "d.rs",
        ),
        (
            "NotebookEdit",
            json!({"notebook_path": "/project/e.ipynb"}),
            ToolKind::Edit,
            "e.ipynb",
        ),
        (
            "Glob",
            json!({"pattern": "*.md"}),
            ToolKind::FindFiles,
            "*.md",
        ),
        (
            "Grep",
            json!({"pattern": "todo"}),
            ToolKind::SearchText,
            "todo",
        ),
        (
            "WebFetch",
            json!({"url": "https://a.b"}),
            ToolKind::Fetch,
            "https://a.b",
        ),
        (
            "WebSearch",
            json!({"query": "rust"}),
            ToolKind::SearchWeb,
            "rust",
        ),
        (
            "Agent",
            json!({"description": "Explore the code"}),
            ToolKind::Delegate,
            "Explore the code",
        ),
        (
            "Task",
            json!({"description": "Plan"}),
            ToolKind::Delegate,
            "Plan",
        ),
        ("SomethingNew", json!({}), ToolKind::Other, "SomethingNew"),
    ];

    for (name, input, kind, subject) in tools {
        assert_eq!(show(name, &input), (kind, subject.to_owned()), "{name}");
    }
}

#[test]
fn shows_an_empty_subject_when_the_input_lacks_it() {
    assert_eq!(
        show("Bash", &json!({"command": 3})),
        (ToolKind::Run, String::new())
    );
    assert_eq!(show("Read", &Value::Null), (ToolKind::Read, String::new()));
}

#[test]
fn shortens_only_paths_inside_the_folder() {
    let folder = Some(Path::new("/project"));

    assert_eq!(relative("/project/src/a.rs", folder), "src/a.rs");
    assert_eq!(relative("/project", folder), "/project");
    assert_eq!(relative("/project/", folder), "/project/");
    assert_eq!(relative("/projector/a.rs", folder), "/projector/a.rs");
    assert_eq!(relative("src/a.rs", folder), "src/a.rs");
    assert_eq!(relative("/project/a.rs", None), "/project/a.rs");
}
