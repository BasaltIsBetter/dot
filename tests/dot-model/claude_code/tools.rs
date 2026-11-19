use serde_json::json;

use super::*;

fn results(line: Value) -> Vec<ToolResult> {
    tool_results(UserLine::deserialize(line).unwrap())
}

#[test]
fn reads_each_tool_result_of_a_line() {
    let line = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "ok"},
        {"type": "text", "text": "aside"},
        {"type": "tool_result", "tool_use_id": "toolu_2", "content": "no", "is_error": true},
    ]}, "parent_tool_use_id": null});

    assert_eq!(
        results(line),
        [
            ToolResult {
                id: "toolu_1".to_owned(),
                is_error: false,
                detail: Detail::Nothing
            },
            ToolResult {
                id: "toolu_2".to_owned(),
                is_error: true,
                detail: Detail::Nothing
            },
        ]
    );
}

#[test]
fn ignores_the_lines_of_a_sub_agent_and_plain_text() {
    let sub_agent = json!({"message": {"content": [
        {"type": "tool_result", "tool_use_id": "toolu_9"}]}, "parent_tool_use_id": "toolu_1"});
    let prompt = json!({"message": {"content": "Salut"}});

    assert_eq!(results(sub_agent), []);
    assert_eq!(results(prompt), []);
}

#[test]
fn measures_what_each_tool_did() {
    let details = [
        (
            json!({"structuredPatch": [{"lines": [" same", "-old", "+new", "+more"]}]}),
            Detail::Changes {
                added: 2,
                removed: 1,
            },
        ),
        (
            json!({"type": "create", "structuredPatch": [], "content": "a\nb\nc"}),
            Detail::Changes {
                added: 3,
                removed: 0,
            },
        ),
        (
            json!({"type": "create", "structuredPatch": [], "content": "a\nb\n"}),
            Detail::Changes {
                added: 2,
                removed: 0,
            },
        ),
        (
            json!({"type": "create", "structuredPatch": [], "content": ""}),
            Detail::Changes {
                added: 0,
                removed: 0,
            },
        ),
        (
            json!({"structuredPatch": []}),
            Detail::Changes {
                added: 0,
                removed: 0,
            },
        ),
        (json!({"file": {"numLines": 42}}), Detail::Lines(42)),
        (json!({"file": {"numLines": "many"}}), Detail::Nothing),
        (
            json!({"numFiles": 3, "numLines": 7, "content": "a:1"}),
            Detail::Matches(7),
        ),
        (json!({"numFiles": 3, "numLines": 7}), Detail::Files(3)),
        (json!({"numFiles": 3}), Detail::Files(3)),
        (json!({"stdout": "hi"}), Detail::Nothing),
        (Value::Null, Detail::Nothing),
    ];

    for (result, detail) in details {
        assert_eq!(detail_of(&result), detail, "{result}");
    }
}
