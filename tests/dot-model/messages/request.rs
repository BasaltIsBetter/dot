use proptest::prelude::*;
use serde_json::{Value, json};

use super::*;
use crate::stream::block::Typed;

const SYSTEM: &str = "Be brief.";

fn text(prompt: &str) -> Prompt {
    Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    }
}

fn body(model: &str, effort: Effort, system: &str, history: &[u8]) -> serde_json::Result<Vec<u8>> {
    let head = head(model, effort, system)?;
    Ok([head.as_slice(), history, MESSAGES_END].concat())
}

fn parsed(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

fn history_of(bytes: &[u8]) -> Value {
    parsed(&[b"[".as_slice(), bytes, b"]"].concat())
}

#[test]
fn asks_for_a_streamed_reply_with_summarized_thinking() {
    let body = parsed(&body("claude-opus-5-5", Effort::High, SYSTEM, b"").unwrap());

    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["max_tokens"], 64_000);
    assert_eq!(body["stream"], true);
    assert_eq!(body["system"], SYSTEM);
    assert_eq!(body["tools"], serde_json::from_str::<Value>(TOOLS).unwrap());
    assert_eq!(
        body["thinking"],
        json!({"type": "adaptive", "display": "summarized",
               "block_binding": {"prefix_mismatch_behavior": "drop_block"}})
    );
    assert_eq!(body["output_config"], json!({"effort": "high"}));
    assert_eq!(body["cache_control"], json!({"type": "ephemeral"}));
    assert_eq!(body["fallbacks"], "default");
    assert_eq!(body["messages"], json!([]));
}

#[test]
fn lets_every_model_but_haiku_fall_back_on_the_server() {
    for model in Model::ALL {
        let body = parsed(&body(model.id, Effort::High, SYSTEM, b"").unwrap());

        if model == Model::HAIKU_5_5 {
            assert!(body.get("fallbacks").is_none(), "{}", model.id);
        } else {
            assert_eq!(body["fallbacks"], "default", "{}", model.id);
        }
    }
}

#[test]
fn asks_every_model_for_the_same_thinking_and_its_effort() {
    for model in Model::ALL {
        let body = parsed(&body(model.id, Effort::Low, SYSTEM, b"").unwrap());

        assert_eq!(body["model"], model.id);
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["output_config"], json!({"effort": "low"}));
    }
}

#[test]
fn sends_no_sampling_parameter() {
    let body = parsed(&body("claude-opus-5-5", Effort::High, SYSTEM, b"").unwrap());

    for parameter in ["temperature", "top_p", "top_k"] {
        assert!(body.get(parameter).is_none());
    }
}

#[test]
fn appends_the_conversation_as_it_was_built() {
    let mut history = Vec::new();
    append_user(&mut history, &text("Hi \"Claude\"\n")).unwrap();
    append_assistant(
        &mut history,
        &[Replayed::Typed(Typed::Text {
            text: "Hello",
            citations: &[],
        })],
    )
    .unwrap();
    append_user(&mut history, &text("Again")).unwrap();

    let body = parsed(&body("claude-opus-5-5", Effort::High, SYSTEM, &history).unwrap());

    assert_eq!(
        body["messages"],
        json!([
            {"role": "user", "content": "Hi \"Claude\"\n"},
            {"role": "assistant", "content": [{"type": "text", "text": "Hello"}]},
            {"role": "user", "content": "Again"},
        ])
    );
}

#[test]
fn appends_nothing_for_an_empty_reply() {
    let mut history = Vec::new();
    append_user(&mut history, &text("Hi")).unwrap();

    append_assistant(&mut history, &[]).unwrap();

    assert_eq!(
        history_of(&history),
        json!([{"role": "user", "content": "Hi"}])
    );
}

#[test]
fn keeps_earlier_turns_byte_for_byte() {
    let mut history = Vec::new();
    append_user(&mut history, &text("First")).unwrap();
    let before = history.clone();

    append_assistant(
        &mut history,
        &[Replayed::Typed(Typed::Text {
            text: "Answer",
            citations: &[],
        })],
    )
    .unwrap();
    append_user(&mut history, &text("Second")).unwrap();

    assert!(history.starts_with(&before));
}

#[test]
fn builds_an_agent_that_never_follows_redirects() {
    assert_eq!(agent().config().max_redirects(), 0);
    assert!(!agent().config().http_status_as_error());
    assert_eq!(agent().config().timeout_per_read(), Some(SILENCE_TIMEOUT));
}

#[test]
fn bounds_every_wait_of_a_request_and_not_only_its_headers() {
    let timeouts = agent().config().timeouts();

    assert_eq!(timeouts.resolve, Some(RESOLVE_TIMEOUT));
    assert_eq!(timeouts.connect, Some(CONNECT_TIMEOUT));
    assert_eq!(timeouts.send_request, Some(SEND_TIMEOUT));
    assert_eq!(agent().config().timeout_per_write(), Some(SEND_TIMEOUT));
    assert_eq!(timeouts.recv_response, Some(ANSWER_TIMEOUT));
}

proptest! {
    #[test]
    fn carries_any_prompt_exactly(prompts in proptest::collection::vec(any::<String>(), 1..5)) {
        let mut history = Vec::new();
        for prompt in &prompts {
            append_user(&mut history, &text(prompt)).unwrap();
        }

        let body = parsed(&body("claude-opus-5-5", Effort::High, SYSTEM, &history).unwrap());

        let sent: Vec<&str> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|message| message["content"].as_str().unwrap())
            .collect();
        let expected: Vec<&str> = prompts.iter().map(String::as_str).collect();
        prop_assert_eq!(sent, expected);
    }
}

#[test]
fn asks_for_the_effort_it_was_given() {
    let body = parsed(&body("claude-opus-5-5", Effort::Low, SYSTEM, b"").unwrap());

    assert_eq!(body["output_config"], json!({"effort": "low"}));
}

#[test]
fn answers_tools_in_one_user_message_and_nothing_when_none_ran() {
    let mut history = Vec::new();
    append_user(&mut history, &text("Look")).unwrap();
    let results = vec![
        (
            "toolu_1".to_owned(),
            dot_api::ToolOutput::done("fn main() {}", dot_api::Detail::Lines(1)),
        ),
        ("toolu_2".to_owned(), dot_api::ToolOutput::refused()),
    ];

    append_tool_results(&mut history, &[]).unwrap();
    append_tool_results(&mut history, &results).unwrap();

    assert_eq!(
        history_of(&history),
        json!([
            {"role": "user", "content": "Look"},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "fn main() {}"},
                {"type": "tool_result", "tool_use_id": "toolu_2",
                 "content": dot_api::USER_REFUSED_THE_TOOL, "is_error": true},
            ]},
        ])
    );
}
