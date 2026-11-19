use std::path::Path;

use super::*;
use crate::stream::block::Typed;

const START: &str = r#"{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":20,"output_tokens":1}}}"#;
const THINKING_START: &str = r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#;
const THINKING_DELTA: &str = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Weighing it."}}"#;
const SIGNATURE: &str = r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}"#;
const THINKING_STOP: &str = r#"{"type":"content_block_stop","index":0}"#;
const TEXT_START: &str =
    r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#;
const HELLO: &str =
    r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Hello"}}"#;
const WORLD: &str =
    r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":" world"}}"#;
const TEXT_STOP: &str = r#"{"type":"content_block_stop","index":1}"#;
const END: &str =
    r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":30}}"#;
const STOP: &str = r#"{"type":"message_stop"}"#;
const REFUSED: &str =
    r#"{"type":"message_delta","delta":{"stop_reason":"refusal"},"usage":{"output_tokens":2}}"#;

fn play(turn: &mut Turn, events: &[&str]) -> Vec<ReplyEvent> {
    let mut heard = Vec::new();
    for event in events {
        turn.apply(serde_json::from_str(event).unwrap(), &mut |event| {
            heard.push(event);
        });
    }
    heard
}

fn whole_reply() -> Vec<&'static str> {
    vec![
        START,
        THINKING_START,
        THINKING_DELTA,
        SIGNATURE,
        THINKING_STOP,
        TEXT_START,
        HELLO,
        WORLD,
        TEXT_STOP,
        END,
        STOP,
    ]
}

#[test]
fn hands_over_thinking_then_text_as_they_arrive() {
    let heard = play(&mut Turn::replayable(None), &whole_reply());

    assert_eq!(
        heard,
        [
            ReplyEvent::Thinking("Weighing it.".into()),
            ReplyEvent::Text("Hello".into()),
            ReplyEvent::Text(" world".into()),
        ]
    );
}

#[test]
fn hands_over_nothing_for_an_empty_thinking_piece() {
    let empty = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":""}}"#;

    let heard = play(&mut Turn::replayable(None), &[START, THINKING_START, empty]);

    assert_eq!(heard, []);
}

#[test]
fn hands_over_the_text_a_block_starts_with() {
    let started =
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":"Hi"}}"#;

    let heard = play(&mut Turn::listening(None), &[START, started]);

    assert_eq!(heard, [ReplyEvent::Text("Hi".into())]);
}

#[test]
fn knows_when_the_message_is_finished_and_what_it_used() {
    let mut turn = Turn::replayable(None);
    play(&mut turn, &whole_reply()[..10]);
    assert!(!turn.is_finished());

    play(&mut turn, &[STOP]);

    assert!(turn.is_finished());
    assert_eq!(turn.model(), "claude-opus-5-5");
    assert_eq!(turn.usage().context_tokens(), 50);
    assert!(!turn.is_declined());
}

#[test]
fn knows_a_declined_reply() {
    let mut turn = Turn::replayable(None);

    play(&mut turn, &[START, TEXT_START, REFUSED, STOP]);

    assert!(turn.is_declined());
}

#[test]
fn replays_thinking_with_its_signature_then_the_text() {
    let mut turn = Turn::replayable(None);
    play(&mut turn, &whole_reply());

    assert_eq!(
        turn.replayed(),
        [
            Replayed::Typed(Typed::Thinking {
                thinking: "Weighing it.",
                signature: "sig",
            }),
            Replayed::Typed(Typed::Text {
                text: "Hello world",
                citations: &[]
            }),
        ]
    );
}

#[test]
fn replays_the_text_written_until_a_cut() {
    let mut turn = Turn::replayable(None);

    play(&mut turn, &whole_reply()[..7]);

    assert_eq!(
        turn.replayed(),
        [
            Replayed::Typed(Typed::Thinking {
                thinking: "Weighing it.",
                signature: "sig",
            }),
            Replayed::Typed(Typed::Text {
                text: "Hello",
                citations: &[]
            }),
        ]
    );
}

#[test]
fn never_replays_unfinished_thinking() {
    let mut turn = Turn::replayable(None);

    play(&mut turn, &[START, THINKING_START, THINKING_DELTA]);

    assert_eq!(turn.replayed(), []);
}

#[test]
fn never_replays_an_empty_text() {
    let mut turn = Turn::replayable(None);

    play(&mut turn, &[START, TEXT_START, TEXT_STOP]);

    assert_eq!(turn.replayed(), []);
}

#[test]
fn drops_the_thinking_before_a_fallback_and_keeps_the_rest_in_place() {
    let fallback = r#"{"type":"content_block_start","index":2,"content_block":{"type":"fallback","from":{"model":"claude-opus-5-5"},"to":{"model":"claude-opus-5"}}}"#;
    let fallback_stop = r#"{"type":"content_block_stop","index":2}"#;
    let continued = r#"{"type":"content_block_start","index":3,"content_block":{"type":"text","text":" again"}}"#;
    let continued_stop = r#"{"type":"content_block_stop","index":3}"#;
    let mut turn = Turn::replayable(None);
    let mut events = whole_reply()[..9].to_vec();
    events.extend([
        fallback,
        fallback_stop,
        continued,
        continued_stop,
        END,
        STOP,
    ]);

    play(&mut turn, &events);

    let replayed = serde_json::to_value(turn.replayed()).unwrap();
    assert_eq!(
        replayed,
        serde_json::json!([
            {"type": "text", "text": "Hello world"},
            {"type": "fallback", "from": {"model": "claude-opus-5-5"}, "to": {"model": "claude-opus-5"}},
            {"type": "text", "text": " again"},
        ])
    );
}

#[test]
fn replays_redacted_thinking_untouched() {
    let redacted = r#"{"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque"}}"#;
    let mut turn = Turn::replayable(None);

    play(&mut turn, &[START, redacted, THINKING_STOP]);

    assert_eq!(
        turn.replayed(),
        [Replayed::Typed(Typed::RedactedThinking { data: "opaque" })]
    );
}

#[test]
fn keeps_nothing_to_replay_when_told_not_to() {
    let mut turn = Turn::listening(None);

    let heard = play(&mut turn, &whole_reply());

    assert_eq!(heard.len(), 3);
    assert_eq!(turn.replayed(), []);
}

#[test]
fn counts_only_the_last_message_of_a_turn() {
    let second = r#"{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":70,"output_tokens":5}}}"#;
    let second_text =
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
    let second_delta =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}"#;
    let mut turn = Turn::listening(None);
    let mut events = whole_reply();
    events.extend([second, second_text, second_delta]);

    let heard = play(&mut turn, &events);

    assert_eq!(turn.usage().context_tokens(), 75);
    assert_eq!(heard.last(), Some(&ReplyEvent::Text("!".into())));
    assert!(!turn.is_finished());
}

#[test]
fn drops_redacted_thinking_before_a_fallback() {
    let redacted = r#"{"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque"}}"#;
    let fallback = r#"{"type":"content_block_start","index":1,"content_block":{"type":"fallback","from":{"model":"a"},"to":{"model":"b"}}}"#;
    let fallback_stop = r#"{"type":"content_block_stop","index":1}"#;
    let mut turn = Turn::replayable(None);

    play(
        &mut turn,
        &[START, redacted, THINKING_STOP, fallback, fallback_stop],
    );

    let replayed = serde_json::to_value(turn.replayed()).unwrap();
    assert_eq!(
        replayed,
        serde_json::json!([{"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}}])
    );
}

const TOOL_START: &str = r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_1","name":"str_replace_based_edit_tool","input":{}}}"#;
const TOOL_INPUT: &str = r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"view\",\"path\":\"/project/src/main.rs\"}"}}"#;
const TOOL_STOP: &str = r#"{"type":"content_block_stop","index":2}"#;
const REDACTED_START: &str = r#"{"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque"}}"#;
const WANTS_TOOLS: &str =
    r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":9}}"#;

fn with_a_tool() -> Vec<&'static str> {
    let mut events = whole_reply();
    events.truncate(9);
    events.extend([TOOL_START, TOOL_INPUT, TOOL_STOP, WANTS_TOOLS, STOP]);
    events
}

#[test]
fn announces_a_tool_once_its_input_is_whole_with_paths_in_the_folder() {
    let mut turn = Turn::replayable(Some(Path::new("/project")));

    let heard = play(&mut turn, &with_a_tool());

    assert_eq!(
        heard.last(),
        Some(&ReplyEvent::ToolStarted {
            id: "toolu_1".to_owned(),
            tool: dot_api::ToolUse {
                kind: dot_api::ToolKind::Read,
                subject: "src/main.rs".to_owned()
            },
        })
    );
    assert!(turn.wants_tools());
    assert!(!turn.is_paused());
    assert_eq!(
        turn.tool_uses(),
        [ToolUseBlock {
            id: "toolu_1",
            name: "str_replace_based_edit_tool",
            input: &serde_json::json!({"command": "view", "path": "/project/src/main.rs"}),
            readable: true,
        }]
    );
}

#[test]
fn replays_a_tool_use_after_the_text() {
    let mut turn = Turn::replayable(None);

    play(&mut turn, &with_a_tool());

    let replayed = serde_json::to_value(turn.replayed()).unwrap();
    assert_eq!(
        replayed[2],
        serde_json::json!({"type": "tool_use", "id": "toolu_1",
            "name": "str_replace_based_edit_tool",
            "input": {"command": "view", "path": "/project/src/main.rs"}})
    );
}

#[test]
fn has_no_tool_to_run_until_its_input_is_whole_nor_for_the_server() {
    let server = r#"{"type":"content_block_start","index":2,"content_block":{"type":"server_tool_use","id":"srv_1","name":"web_search","input":{}}}"#;
    let mut unfinished = Turn::replayable(None);
    let mut on_the_server = Turn::replayable(None);

    play(&mut unfinished, &with_a_tool()[..11]);
    play(&mut on_the_server, &[START, server, TOOL_STOP]);

    assert_eq!(unfinished.tool_uses(), []);
    assert_eq!(on_the_server.tool_uses(), []);
    assert!(!unfinished.wants_tools());
}

#[test]
fn knows_a_paused_turn() {
    let paused = r#"{"type":"message_delta","delta":{"stop_reason":"pause_turn"},"usage":{"output_tokens":2}}"#;
    let mut turn = Turn::replayable(None);

    play(&mut turn, &[START, paused]);

    assert!(turn.is_paused());
    assert!(!turn.wants_tools());
    assert!(!turn.is_declined());
}

#[test]
fn keeps_the_citations_a_text_receives() {
    let cited = r#"{"type":"content_block_delta","index":1,"delta":{"type":"citations_delta","citation":{"url":"https://docs.rs"}}}"#;
    let mut turn = Turn::replayable(None);

    play(
        &mut turn,
        &[START, REDACTED_START, TEXT_START, HELLO, cited, TEXT_STOP],
    );

    let replayed = serde_json::to_value(turn.replayed()).unwrap();
    assert_eq!(
        replayed,
        serde_json::json!([{"type": "text", "text": "Hello",
            "citations": [{"url": "https://docs.rs"}]}])
    );
}

#[test]
fn ignores_input_and_citations_for_a_block_that_takes_none() {
    let stray_input = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#;
    let stray_citation = r#"{"type":"content_block_delta","index":0,"delta":{"type":"citations_delta","citation":{}}}"#;
    let mut turn = Turn::replayable(None);

    let heard = play(
        &mut turn,
        &[
            START,
            THINKING_START,
            TEXT_START,
            stray_input,
            stray_citation,
            HELLO,
        ],
    );

    assert_eq!(heard, [ReplyEvent::Text("Hello".into())]);
}
