use serde_json::json;

use super::*;
use crate::tool::tests::FakeToolbox;

fn request(tool: &str, input: Value) -> ControlRequest {
    ControlRequest {
        request_id: Value::from("req-1"),
        request: Request {
            subtype: CAN_USE_TOOL.to_owned(),
            tool_name: tool.to_owned(),
            input,
            tool_use_id: "toolu_1".to_owned(),
            description: Some("What it does".to_owned()),
        },
    }
}

fn action(tool: &str, input: Value) -> Action {
    request(tool, input)
        .permission(&FakeToolbox::in_folder("/project"))
        .unwrap()
        .action
}

fn owned(text: &str) -> String {
    text.to_owned()
}

#[test]
fn asks_about_what_each_tool_of_claude_code_would_do() {
    let actions = [
        (
            "Bash",
            json!({"command": "rm -rf build"}),
            Action::Run {
                command: owned("rm -rf build"),
            },
        ),
        (
            "Write",
            json!({"file_path": "/project/a.rs"}),
            Action::Change {
                path: owned("a.rs"),
            },
        ),
        (
            "Edit",
            json!({"file_path": "/project/src/b.rs"}),
            Action::Change {
                path: owned("src/b.rs"),
            },
        ),
        (
            "MultiEdit",
            json!({"file_path": "/etc/hosts"}),
            Action::ChangeOutside {
                path: owned("/etc/hosts"),
            },
        ),
        (
            "NotebookEdit",
            json!({"notebook_path": "/project/n.ipynb"}),
            Action::Change {
                path: owned("n.ipynb"),
            },
        ),
        (
            "Read",
            json!({"file_path": "/etc/passwd"}),
            Action::ReadOutside {
                path: owned("/etc/passwd"),
            },
        ),
        (
            "Glob",
            json!({"path": "/Users"}),
            Action::ReadOutside {
                path: owned("/Users"),
            },
        ),
        (
            "Grep",
            json!({"path": "/var"}),
            Action::ReadOutside {
                path: owned("/var"),
            },
        ),
        (
            "WebFetch",
            json!({"url": "https://a.b"}),
            Action::Fetch {
                url: owned("https://a.b"),
            },
        ),
        (
            "WebSearch",
            json!({"query": "rust"}),
            Action::SearchWeb {
                query: owned("rust"),
            },
        ),
        (
            "Agent",
            json!({}),
            Action::Use {
                tool: owned("Agent"),
                detail: owned("What it does"),
            },
        ),
        (
            "Write",
            json!({}),
            Action::Use {
                tool: owned("Write"),
                detail: owned("What it does"),
            },
        ),
    ];

    for (tool, input, expected) in actions {
        assert_eq!(action(tool, input), expected, "{tool}");
    }
}

#[test]
fn asks_as_the_tool_call_that_wants_it() {
    let permission = request("Bash", json!({"command": "ls"}))
        .permission(&FakeToolbox::without_folder())
        .unwrap();

    assert_eq!(permission.tool_id, "toolu_1");
}

#[test]
fn asks_nothing_for_another_kind_of_request() {
    let mut other = request("Bash", json!({}));
    other.request.subtype = "hook_callback".to_owned();

    assert_eq!(other.permission(&FakeToolbox::without_folder()), None);
}

fn answered(decision: Option<Decision>) -> Value {
    let mut written = Vec::new();
    request("Bash", json!({"command": "ls"}))
        .answer(decision, &mut written)
        .unwrap();
    assert_eq!(written.last(), Some(&b'\n'));
    serde_json::from_slice(&written).unwrap()
}

#[test]
fn answers_on_one_line_as_claude_code_expects() {
    assert_eq!(
        answered(Some(Decision::Allow)),
        json!({"type": "control_response", "response": {"subtype": "success",
            "request_id": "req-1",
            "response": {"behavior": "allow", "updatedInput": {"command": "ls"}}}})
    );
    assert_eq!(
        answered(Some(Decision::Deny)),
        json!({"type": "control_response", "response": {"subtype": "success",
            "request_id": "req-1",
            "response": {"behavior": "deny", "message": USER_REFUSED_THE_TOOL,
                "interrupt": false}}})
    );
    assert_eq!(
        answered(None),
        json!({"type": "control_response", "response": {"subtype": "error",
            "request_id": "req-1", "error": DOT_DOES_NOT_HANDLE_THIS_REQUEST}})
    );
}

#[test]
fn answers_with_the_id_claude_code_gave_whatever_its_type() {
    let mut asked = request("Bash", json!({"command": "ls"}));
    asked.request_id = json!(5);
    let mut written = Vec::new();

    asked.answer(None, &mut written).unwrap();

    let answered: Value = serde_json::from_slice(&written).unwrap();
    assert_eq!(answered["response"]["request_id"], json!(5));
}
