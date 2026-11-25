use super::*;

fn read(json: &Value) -> Request {
    serde_json::from_value(json.clone()).unwrap()
}

#[test]
fn reads_a_call_to_an_agent() {
    let request = read(&json!({
        "jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": {"name": "call_agent", "arguments": {"agent": "reviewer", "prompt": "Look", "continue": true}},
    }));

    assert_eq!(request.id, Some(json!(7)));
    assert_eq!(
        request.asked,
        Asked::CallAgent(CallArguments {
            agent: "reviewer".to_owned(),
            prompt: "Look".to_owned(),
            goes_on: true,
        })
    );
}

#[test]
fn a_call_without_a_prompt_is_malformed() {
    let request = read(&json!({
        "id": 7, "method": "tools/call",
        "params": {"name": "call_agent", "arguments": {"agent": "reviewer"}},
    }));

    assert!(matches!(request.asked, Asked::Malformed(_)));
}

#[test]
fn speaks_the_version_it_is_asked_for() {
    let asked = |params| read(&json!({"id": 1, "method": "initialize", "params": params})).asked;

    assert_eq!(
        asked(json!({"protocolVersion": "2024-11-05"})),
        Asked::Initialize("2024-11-05".to_owned())
    );
    assert_eq!(
        asked(Value::Null),
        Asked::Initialize(DEFAULT_PROTOCOL.to_owned())
    );
}

#[test]
fn reads_what_is_no_request() {
    let cancelled = read(&json!({"method": "notifications/cancelled", "params": {"requestId": 3}}));

    assert_eq!(cancelled.id, None);
    assert_eq!(cancelled.asked, Asked::Cancelled(json!(3)));
    assert_eq!(read(&json!({"method": "other"})).asked, Asked::Unknown);
}

#[test]
fn offers_two_tools() {
    let tools = tools();
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();

    assert_eq!(names, [LIST_AGENTS, CALL_AGENT]);
}
