use serde_json::json;

use super::*;

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("not an object");
    };
    map
}

fn tool(written: &str) -> ToolUse {
    ToolUse {
        id: "toolu_1".to_owned(),
        name: "bash".to_owned(),
        server: false,
        written: written.to_owned(),
        input: Value::Object(Map::new()),
        readable: true,
    }
}

#[test]
fn reads_the_input_once_it_is_written_whole() {
    let mut written = tool(r#"{"command":"ls"}"#);
    let mut empty = tool("  ");
    let mut broken = tool(r#"{"command":"#);
    let mut not_an_object = tool("[1]");

    for tool in [&mut written, &mut empty, &mut broken, &mut not_an_object] {
        tool.finish();
    }

    assert_eq!(written.input, json!({"command": "ls"}));
    assert!(written.readable);
    assert_eq!(empty.input, json!({}));
    assert!(empty.readable);
    assert!(!broken.readable);
    assert!(!not_an_object.readable);
}

#[test]
fn ends_a_server_tool_from_its_result() {
    let ends = [
        (
            json!({"content": [{}, {}]}),
            ToolEnd::Done(Detail::Matches(2)),
        ),
        (
            json!({"content": {"error_code": "max_uses_exceeded"}}),
            ToolEnd::Failed,
        ),
        (
            json!({"content": {"return_code": 3}}),
            ToolEnd::Done(Detail::Exit(3)),
        ),
        (
            json!({"content": {"return_code": 9_000_000_000_i64}}),
            ToolEnd::Done(Detail::Nothing),
        ),
        (
            json!({"content": {"type": "web_fetch_result"}}),
            ToolEnd::Done(Detail::Nothing),
        ),
        (json!({}), ToolEnd::Done(Detail::Nothing)),
    ];

    for (result, end) in ends {
        assert_eq!(server_end(&map(result.clone())), end, "{result}");
    }
}

#[test]
fn announces_the_end_of_a_server_tool_as_its_result_arrives() {
    let mut heard = Vec::new();
    let result = json!({"type": "web_search_tool_result", "tool_use_id": "srv_1", "content": []});
    let anonymous = json!({"type": "something_new"});

    let kept = Content::of(StartedBlock::Raw(map(result)), true, &mut |event| {
        heard.push(event);
    });
    let dropped = Content::of(StartedBlock::Raw(map(anonymous)), false, &mut |event| {
        heard.push(event);
    });

    assert_eq!(
        heard,
        [ReplyEvent::ToolEnded {
            id: "srv_1".to_owned(),
            end: ToolEnd::Done(Detail::Matches(0)),
        }]
    );
    assert!(matches!(kept, Content::Raw(_)));
    assert!(matches!(dropped, Content::Other));
}

#[test]
fn keeps_a_tool_use_even_when_nothing_else_is_kept() {
    let content = Content::of(
        StartedBlock::ToolUse {
            id: "toolu_1".to_owned(),
            name: "Bash".to_owned(),
            server: false,
        },
        false,
        &mut |_| {},
    );

    assert!(matches!(content, Content::ToolUse(_)));
}

#[test]
fn replays_a_tool_use_only_once_it_is_whole() {
    let mut whole = Block {
        content: Content::ToolUse(tool(r#"{"command":"ls"}"#)),
        complete: true,
    };
    let cut = Block {
        content: Content::ToolUse(tool(r#"{"comm"#)),
        complete: false,
    };
    if let Content::ToolUse(tool) = &mut whole.content {
        tool.finish();
    }

    assert!(whole.replayed(false).is_some());
    assert!(cut.replayed(false).is_none());
}

#[test]
fn replays_a_result_block_untouched_even_before_a_fallback() {
    let raw = Block {
        content: Content::Raw(map(json!({"type": "web_fetch_tool_result"}))),
        complete: true,
    };

    assert!(matches!(raw.replayed(true), Some(Replayed::Raw(_))));
}
