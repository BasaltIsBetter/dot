use proptest::prelude::*;
use serde_json::json;

use super::*;

fn block(value: Value) -> StartedBlock {
    let Value::Object(map) = value else {
        panic!("not an object");
    };
    StartedBlock::of(map)
}

#[test]
fn reads_each_kind_of_block_it_knows() {
    assert_eq!(
        block(json!({"type": "text", "text": "Hi"})),
        StartedBlock::Text {
            text: "Hi".to_owned(),
            citations: Vec::new()
        }
    );
    assert_eq!(
        block(json!({"type": "text", "text": "", "citations": [{"url": "u"}]})),
        StartedBlock::Text {
            text: String::new(),
            citations: vec![json!({"url": "u"})]
        }
    );
    assert_eq!(
        block(json!({"type": "thinking", "thinking": "", "signature": ""})),
        StartedBlock::Thinking {
            thinking: String::new(),
            signature: String::new()
        }
    );
    assert_eq!(
        block(json!({"type": "redacted_thinking", "data": "abc"})),
        StartedBlock::RedactedThinking {
            data: "abc".to_owned()
        }
    );
    assert_eq!(
        block(json!({"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {}})),
        StartedBlock::ToolUse {
            id: "toolu_1".to_owned(),
            name: "bash".to_owned(),
            server: false
        }
    );
    assert_eq!(
        block(json!({"type": "server_tool_use", "id": "srv_1", "name": "web_search", "input": {}})),
        StartedBlock::ToolUse {
            id: "srv_1".to_owned(),
            name: "web_search".to_owned(),
            server: true
        }
    );
}

#[test]
fn keeps_a_fallback_and_any_other_block_whole() {
    let fallback = json!({"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}});
    let result = json!({"type": "web_fetch_tool_result", "tool_use_id": "srv_1", "content": {}});
    let untyped = json!({"text": "no type"});

    assert_eq!(
        block(fallback.clone()),
        StartedBlock::Fallback(fallback.as_object().unwrap().clone())
    );
    assert_eq!(
        block(result.clone()),
        StartedBlock::Raw(result.as_object().unwrap().clone())
    );
    assert_eq!(
        block(untyped.clone()),
        StartedBlock::Raw(untyped.as_object().unwrap().clone())
    );
}

#[test]
fn keeps_a_known_block_it_cannot_read_whole() {
    let broken = json!({"type": "tool_use", "name": 3});

    assert_eq!(
        block(broken.clone()),
        StartedBlock::Raw(broken.as_object().unwrap().clone())
    );
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

        let _ = StartedBlock::of(map);
    }
}
