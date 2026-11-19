use proptest::prelude::*;
use serde_json::json;

use super::*;

fn call(name: &str, input: &Value) -> Result<ToolCall, String> {
    call_of(name, input)
}

#[test]
fn reads_the_bash_tool() {
    assert_eq!(
        call("bash", &json!({"command": "ls -la"})),
        Ok(ToolCall::Run {
            command: "ls -la".to_owned()
        })
    );
    assert_eq!(
        call("bash", &json!({"restart": true, "command": "ls"})),
        Ok(ToolCall::RestartShell)
    );
    assert_eq!(call("bash", &json!({})), Err(NO_COMMAND.to_owned()));
}

#[test]
fn reads_each_command_of_the_text_editor() {
    let editor = |input: Value| call("str_replace_based_edit_tool", &input);

    assert_eq!(
        editor(json!({"command": "view", "path": "/a.rs"})),
        Ok(ToolCall::View {
            path: "/a.rs".to_owned(),
            lines: None
        })
    );
    assert_eq!(
        editor(json!({"command": "view", "path": "/a.rs", "view_range": [3, -1]})),
        Ok(ToolCall::View {
            path: "/a.rs".to_owned(),
            lines: Some(LineRange {
                first: 3,
                last: None
            })
        })
    );
    assert_eq!(
        editor(json!({"command": "view", "path": "/a.rs", "view_range": [3, 3]})),
        Ok(ToolCall::View {
            path: "/a.rs".to_owned(),
            lines: Some(LineRange {
                first: 3,
                last: Some(3)
            })
        })
    );
    assert_eq!(
        editor(json!({"command": "create", "path": "/b.rs", "file_text": "x\n"})),
        Ok(ToolCall::Create {
            path: "/b.rs".to_owned(),
            text: "x\n".to_owned()
        })
    );
    assert_eq!(
        editor(json!({"command": "str_replace", "path": "/c.rs", "old_str": "a"})),
        Ok(ToolCall::Replace {
            path: "/c.rs".to_owned(),
            old: "a".to_owned(),
            new: String::new()
        })
    );
    assert_eq!(
        editor(json!({"command": "insert", "path": "/d.rs", "insert_line": 0, "insert_text": "y"})),
        Ok(ToolCall::Insert {
            path: "/d.rs".to_owned(),
            after_line: 0,
            text: "y".to_owned()
        })
    );
    assert_eq!(
        editor(json!({"command": "insert", "path": "/d.rs", "insert_line": 2, "new_str": "z"})),
        Ok(ToolCall::Insert {
            path: "/d.rs".to_owned(),
            after_line: 2,
            text: "z".to_owned()
        })
    );
    assert_eq!(
        editor(json!({"command": "undo_edit", "path": "/d.rs"})),
        Err(UNDO_UNSUPPORTED.to_owned())
    );
}

#[test]
fn refuses_a_range_that_is_not_one() {
    let editor = |range: Value| {
        call(
            "str_replace_based_edit_tool",
            &json!({"command": "view", "path": "/a.rs", "view_range": range}),
        )
    };

    for range in [json!([0, 4]), json!([-2, 4]), json!([5, 4]), json!([1, -2])] {
        assert_eq!(editor(range.clone()), Err(BAD_RANGE.to_owned()), "{range}");
    }
}

#[test]
fn reads_the_search_tools() {
    assert_eq!(
        call("find_files", &json!({"pattern": "**/*.rs", "path": "/src"})),
        Ok(ToolCall::FindFiles {
            pattern: "**/*.rs".to_owned(),
            folder: Some("/src".to_owned())
        })
    );
    assert_eq!(
        call("search_text", &json!({"pattern": "fn"})),
        Ok(ToolCall::SearchText(TextSearch {
            pattern: "fn".to_owned(),
            folder: None,
            files: None,
            ignore_case: false,
            show: SearchShow::Files,
        }))
    );
    assert_eq!(
        call(
            "search_text",
            &json!({"pattern": "fn", "path": "/p", "glob": "*.rs", "ignore_case": true, "output": "lines"})
        ),
        Ok(ToolCall::SearchText(TextSearch {
            pattern: "fn".to_owned(),
            folder: Some("/p".to_owned()),
            files: Some("*.rs".to_owned()),
            ignore_case: true,
            show: SearchShow::Lines,
        }))
    );
    assert!(matches!(
        call("search_text", &json!({"pattern": "fn", "output": "counts"})),
        Ok(ToolCall::SearchText(TextSearch {
            show: SearchShow::Counts,
            ..
        }))
    ));
}

#[test]
fn refuses_an_unknown_tool_or_a_malformed_input() {
    assert_eq!(call("rm_rf", &json!({})), Err(UNKNOWN_TOOL.to_owned()));
    assert_eq!(
        call("Bash", &json!({"command": "ls"})),
        Err(UNKNOWN_TOOL.to_owned())
    );
    let malformed = call("str_replace_based_edit_tool", &json!({"command": "view"}));
    assert!(
        malformed
            .unwrap_err()
            .starts_with("The tool input is not valid: ")
    );
    assert!(call("find_files", &json!({"pattern": 3})).is_err());
    assert!(call("search_text", &json!({"pattern": "a", "output": "all"})).is_err());
}

proptest! {
    #[test]
    fn never_panics_on_any_input(
        name in prop::sample::select(vec!["bash", "str_replace_based_edit_tool", "find_files", "search_text", "x"]),
        command in "[a-z_]{0,12}",
        path in "\\PC{0,20}",
        first in any::<i64>(),
        last in any::<i64>(),
    ) {
        let input = json!({"command": command, "path": path, "view_range": [first, last],
            "pattern": path, "insert_line": first, "insert_text": path});

        let _ = call_of(name, &input);
    }
}
