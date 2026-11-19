use proptest::prelude::*;
use serde_json::json;

use super::*;

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("not an object");
    };
    map
}

fn read(value: Value) -> Option<Known> {
    Known::read(&map(value))
}

#[test]
fn reads_each_kind_of_block_it_knows() {
    assert_eq!(
        read(json!({"type": "text", "text": "Hi"})),
        Some(Known::Text {
            text: "Hi".to_owned(),
            citations: None
        })
    );
    assert_eq!(
        read(json!({"type": "text", "citations": [{"url": "u"}]})),
        Some(Known::Text {
            text: String::new(),
            citations: Some(vec![json!({"url": "u"})])
        })
    );
    assert_eq!(
        read(json!({"type": "thinking"})),
        Some(Known::Thinking {
            thinking: String::new(),
            signature: String::new()
        })
    );
    assert_eq!(
        read(json!({"type": "redacted_thinking", "data": "abc"})),
        Some(Known::RedactedThinking {
            data: "abc".to_owned()
        })
    );
    assert_eq!(
        read(json!({"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {}})),
        Some(Known::ToolUse {
            id: "toolu_1".to_owned(),
            name: "bash".to_owned()
        })
    );
    assert_eq!(
        read(json!({"type": "server_tool_use", "id": "srv_1", "name": "web_search", "input": {}})),
        Some(Known::ServerToolUse {
            id: "srv_1".to_owned(),
            name: "web_search".to_owned()
        })
    );
}

#[test]
fn reads_nothing_from_a_fallback_or_any_other_block() {
    let fallback = json!({"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}});
    let result = json!({"type": "web_fetch_tool_result", "tool_use_id": "srv_1", "content": {}});
    let untyped = json!({"text": "no type"});

    for other in [fallback, result, untyped] {
        assert_eq!(read(other.clone()), None, "{other}");
    }
}

#[test]
fn reads_nothing_from_a_known_block_that_lacks_a_field() {
    assert_eq!(read(json!({"type": "tool_use", "name": 3})), None);
    assert_eq!(read(json!({"type": "redacted_thinking"})), None);
}

#[test]
fn tells_a_fallback_from_any_other_block() {
    assert!(is_fallback(&map(json!({"type": "fallback"}))));
    assert!(!is_fallback(&map(
        json!({"type": "text", "text": "fallback"})
    )));
    assert!(!is_fallback(&map(json!({"type": 3}))));
    assert!(!is_fallback(&map(json!({}))));
}

#[test]
fn replays_typed_blocks_and_others_untouched() {
    let input = json!({"command": "ls"});
    let raw = json!({"type": "web_search_tool_result", "tool_use_id": "srv_1", "content": []});
    let replayed = [
        Replayed::Typed(Typed::Text {
            text: "Hi",
            citations: &[],
        }),
        Replayed::Typed(Typed::Text {
            text: "Cited",
            citations: &[json!({"url": "u"})],
        }),
        Replayed::Typed(Typed::tool_use("toolu_1", "bash", &input, false)),
        Replayed::Typed(Typed::tool_use("srv_1", "web_search", &input, true)),
        Replayed::Raw(raw.as_object().unwrap()),
    ];

    assert_eq!(
        serde_json::to_value(replayed).unwrap(),
        json!([
            {"type": "text", "text": "Hi"},
            {"type": "text", "text": "Cited", "citations": [{"url": "u"}]},
            {"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {"command": "ls"}},
            {"type": "server_tool_use", "id": "srv_1", "name": "web_search",
             "input": {"command": "ls"}},
            {"type": "web_search_tool_result", "tool_use_id": "srv_1", "content": []},
        ])
    );
}

proptest! {
    #[test]
    fn never_panics_on_any_block(text in "\\PC{0,40}", kind in "[a-z_]{0,20}") {
        let value = json!({"type": kind, "text": text, "id": text, "name": kind});
        let Value::Object(map) = value else { unreachable!("built as an object") };

        let known = Known::read(&map);

        prop_assert_eq!(is_fallback(&map), kind == "fallback");
        prop_assert!(known.is_none() || !is_fallback(&map));
    }
}
