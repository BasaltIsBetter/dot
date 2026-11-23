use serde_json::json;

use super::*;

fn results(line: Value) -> Vec<ToolResult> {
    tool_results(UserLine::deserialize(line).unwrap()).1
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
                detail: Detail::Nothing,
                agent_tokens: None,
                change: None,
            },
            ToolResult {
                id: "toolu_2".to_owned(),
                is_error: true,
                detail: Detail::Nothing,
                agent_tokens: None,
                change: None,
            },
        ]
    );
}

#[test]
fn tells_the_results_of_a_sub_agent_by_the_call_that_started_it() {
    let sub_agent = json!({"message": {"content": [
        {"type": "tool_result", "tool_use_id": "toolu_9"}]}, "parent_tool_use_id": "toolu_1"});
    let prompt = json!({"message": {"content": "Salut"}, "parent_tool_use_id": "toolu_1"});

    let (agent, results) = tool_results(UserLine::deserialize(sub_agent).unwrap());
    let (_, nothing) = tool_results(UserLine::deserialize(prompt).unwrap());

    assert_eq!(agent.as_deref(), Some("toolu_1"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "toolu_9");
    assert_eq!(nothing, []);
}

#[test]
fn reads_what_an_agent_wrote_from_its_hand_back() {
    let handed_back = json!({"message": {"content": [
        {"type": "tool_result", "tool_use_id": "toolu_1", "content": []}]},
        "parent_tool_use_id": null,
        "tool_use_result": {"status": "completed", "agentType": "general-purpose",
            "usage": {"input_tokens": 2, "output_tokens": 243}}});
    let ordinary = json!({"message": {"content": [
        {"type": "tool_result", "tool_use_id": "toolu_2"}]},
        "tool_use_result": {"usage": {"output_tokens": 243}}});

    assert_eq!(results(handed_back)[0].agent_tokens, Some(243));
    assert_eq!(results(ordinary)[0].agent_tokens, None);
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

#[test]
fn reads_the_change_of_the_one_result_of_a_line() {
    let patch = json!({"filePath": "/p/a.rs", "structuredPatch":
        [{"oldStart": 1, "newStart": 1, "lines": ["-a", "+b"]}]});
    let one = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "ok"}]},
        "tool_use_result": patch.clone()});
    let failed = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "toolu_1", "content": "no", "is_error": true}]},
        "tool_use_result": patch.clone()});
    let two = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "toolu_1"},
        {"type": "tool_result", "tool_use_id": "toolu_2"}]},
        "tool_use_result": patch});

    let read = results(one);

    assert_eq!(read[0].change.as_ref().unwrap().path, "/p/a.rs");
    assert_eq!(results(failed)[0].change, None);
    assert!(results(two).iter().all(|result| result.change.is_none()));
}
