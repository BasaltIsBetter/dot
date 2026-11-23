use proptest::prelude::*;

use super::*;

fn event(json: &str) -> Event {
    serde_json::from_str(json).unwrap()
}

#[test]
fn reads_the_start_of_a_message_with_its_model_and_usage() {
    let Event::MessageStart { message } = event(
        r#"{"type":"message_start","message":{"id":"msg_1","model":"claude-opus-5-5",
            "usage":{"input_tokens":12,"cache_read_input_tokens":3,"output_tokens":1}}}"#,
    ) else {
        panic!("not a message start");
    };

    assert_eq!(message.model, "claude-opus-5-5");
    assert_eq!(message.usage.input(), 12);
    assert_eq!(message.usage.context_tokens(), 16);
}

#[test]
fn keeps_a_started_block_whole_to_read_it_later() {
    let Event::ContentBlockStart { content_block } = event(
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"web_search_tool_result","tool_use_id":"srvtoolu_1","content":[]}}"#,
    ) else {
        panic!("not a block start");
    };

    assert_eq!(content_block["type"], "web_search_tool_result");
    assert_eq!(content_block["tool_use_id"], "srvtoolu_1");
}

#[test]
fn reads_each_kind_of_delta() {
    let delta = |json: &str| match event(json) {
        Event::ContentBlockDelta { delta, .. } => delta,
        other => panic!("not a delta: {other:?}"),
    };

    assert!(matches!(
        delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#),
        Delta::Text { text } if text == "Hi"
    ));
    assert!(matches!(
        delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Hm"}}"#),
        Delta::Thinking { thinking } if thinking == "Hm"
    ));
    assert!(matches!(
        delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"s"}}"#),
        Delta::Signature { signature } if signature == "s"
    ));
    assert!(matches!(
        delta(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"a\""}}"#
        ),
        Delta::InputJson { partial_json } if partial_json == "{\"a\""
    ));
    assert!(matches!(
        delta(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"citations_delta","citation":{"url":"u"}}}"#
        ),
        Delta::Citation { citation } if citation["url"] == "u"
    ));
    assert!(matches!(
        delta(r#"{"type":"content_block_delta","index":0,"delta":{"type":"new_delta"}}"#),
        Delta::Other
    ));
}

#[test]
fn ignores_pings_and_events_it_does_not_know() {
    assert!(matches!(event(r#"{"type":"ping"}"#), Event::Other));
    assert!(matches!(
        event(r#"{"type":"something_new","x":1}"#),
        Event::Other
    ));
}

#[test]
fn reads_an_error_in_the_stream() {
    let Event::Error { error } =
        event(r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#)
    else {
        panic!("not an error");
    };

    assert_eq!(error.failure(), ReplyFailure::Overloaded);
    assert_eq!(
        error.to_string(),
        "Anthropic answered with overloaded_error: Overloaded"
    );
}

#[test]
fn tells_each_kind_of_error_apart() {
    let kinds = [
        ("overloaded_error", ReplyFailure::Overloaded),
        ("rate_limit_error", ReplyFailure::RateLimited),
        ("authentication_error", ReplyFailure::Unauthorized),
        ("permission_error", ReplyFailure::Unauthorized),
        ("invalid_request_error", ReplyFailure::Rejected),
        ("not_found_error", ReplyFailure::Rejected),
        ("request_too_large", ReplyFailure::Rejected),
        ("api_error", ReplyFailure::Unreachable),
        ("anything_else", ReplyFailure::Unreachable),
    ];

    for (kind, failure) in kinds {
        assert_eq!(failure_of_kind(kind), failure, "{kind}");
    }
}

#[test]
fn merges_the_counts_a_later_event_gives() {
    let mut usage: Usage = serde_json::from_str(
        r#"{"input_tokens":10,"cache_creation_input_tokens":5,"output_tokens":1}"#,
    )
    .unwrap();
    let later: Usage = serde_json::from_str(r#"{"output_tokens":40,"input_tokens":null}"#).unwrap();

    usage.merge(later);

    assert_eq!(usage.input(), 10);
    assert_eq!(usage.output(), 40);
    assert_eq!(usage.cache_written(), 5);
    assert_eq!(usage.context_tokens(), 55);
}

#[test]
fn never_counts_more_hour_long_writes_than_writes() {
    let usage: Usage = serde_json::from_str(
        r#"{"cache_creation_input_tokens":5,"cache_creation":{"ephemeral_1h_input_tokens":9}}"#,
    )
    .unwrap();

    assert_eq!(usage.cache_written_for_an_hour(), 5);
}

proptest! {
    #[test]
    fn reads_anything_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        if let Ok(Event::MessageStart { message }) = serde_json::from_slice::<Event>(&bytes) {
            prop_assert!(message.usage.context_tokens() >= message.usage.input());
        }
    }

    #[test]
    fn hands_back_any_text_delta(text in any::<String>()) {
        let json = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": text},
        });

        let parsed: Event = serde_json::from_value(json).unwrap();

        let Event::ContentBlockDelta { delta: Delta::Text { text: read }, .. } = parsed else {
            panic!("not a text delta");
        };
        prop_assert_eq!(read, text);
    }
}

#[test]
fn counts_what_claude_read_apart_from_what_it_wrote() {
    let Event::MessageStart { message } = event(
        r#"{"type":"message_start","message":{"model":"claude-opus-5-5",
            "usage":{"input_tokens":10,"cache_read_input_tokens":20,"cache_creation_input_tokens":5,"output_tokens":3}}}"#,
    ) else {
        panic!("not a message start");
    };

    assert_eq!(message.usage.prompt_tokens(), 35);
    assert_eq!(message.usage.context_tokens(), 38);
}
