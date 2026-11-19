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

    let kept = Content::of(map(result), true, &mut |event| {
        heard.push(event);
    });
    let dropped = Content::of(map(anonymous), false, &mut |event| {
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
    let started = json!({"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {}});

    let content = Content::of(map(started), false, &mut |_| {});

    assert!(matches!(content, Content::ToolUse(_)));
}

#[test]
fn starts_a_text_with_what_the_block_already_holds_and_hands_it_over() {
    let mut heard = Vec::new();
    let cited = json!({"type": "text", "text": "Hi", "citations": [{"url": "u"}]});

    let content = Content::of(map(cited), true, &mut |event| heard.push(event));
    let empty = Content::of(map(json!({"type": "text"})), true, &mut |event| {
        heard.push(event);
    });

    assert_eq!(heard, [ReplyEvent::Text("Hi".to_owned())]);
    let Content::Text { text, citations } = content else {
        panic!("not a text");
    };
    assert_eq!(
        (text.as_str(), citations),
        ("Hi", vec![json!({"url": "u"})])
    );
    assert!(
        matches!(empty, Content::Text { text, citations } if text.is_empty() && citations.is_empty())
    );
}

#[test]
fn starts_thinking_and_hands_over_only_what_is_written() {
    let mut heard = Vec::new();
    let started = json!({"type": "thinking", "thinking": "Hm", "signature": ""});

    let content = Content::of(map(started), true, &mut |event| heard.push(event));
    Content::of(map(json!({"type": "thinking"})), true, &mut |event| {
        heard.push(event);
    });
    let redacted = Content::of(
        map(json!({"type": "redacted_thinking", "data": "abc"})),
        true,
        &mut |event| heard.push(event),
    );

    assert_eq!(heard, [ReplyEvent::Thinking("Hm".to_owned())]);
    assert!(matches!(content, Content::Thinking { thinking, .. } if thinking == "Hm"));
    assert!(matches!(redacted, Content::RedactedThinking(data) if data == "abc"));
}

#[test]
fn starts_a_tool_of_dot_or_of_the_server_with_an_empty_input() {
    let own = json!({"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {}});
    let server = json!({"type": "server_tool_use", "id": "srv_1", "name": "web_search"});

    let Content::ToolUse(own) = Content::of(map(own), true, &mut |_| {}) else {
        panic!("not a tool use");
    };
    let Content::ToolUse(server) = Content::of(map(server), true, &mut |_| {}) else {
        panic!("not a tool use");
    };

    assert_eq!((own.id.as_str(), own.name.as_str()), ("toolu_1", "bash"));
    assert!(!own.server);
    assert!(own.readable);
    assert_eq!(own.input, json!({}));
    assert_eq!(own.written, "");
    assert_eq!(
        (server.id.as_str(), server.name.as_str()),
        ("srv_1", "web_search")
    );
    assert!(server.server);
}

#[test]
fn keeps_a_fallback_and_a_block_it_cannot_read_whole() {
    let fallback = json!({"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}});
    let broken = json!({"type": "tool_use", "name": 3});
    let mut heard = Vec::new();

    let kept = Content::of(map(fallback.clone()), true, &mut |event| heard.push(event));
    let raw = Content::of(map(broken.clone()), true, &mut |event| heard.push(event));

    let Content::Fallback(kept) = kept else {
        panic!("not a fallback");
    };
    let Content::Raw(raw) = raw else {
        panic!("not kept whole");
    };
    assert_eq!(Value::Object(kept), fallback);
    assert_eq!(Value::Object(raw), broken);
    assert_eq!(heard, []);
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
