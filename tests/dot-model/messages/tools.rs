use dot_api::{Cancellation, Decision, Detail, ToolCall};

use super::*;
use crate::stream::Turn;
use crate::tool::tests::{Answering, FakeToolbox};

const MESSAGE: &str = r#"{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":1}}}"#;
const END: &str =
    r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":4}}"#;

fn turn_calling(tools: &[(&str, &str, &str)]) -> Turn<'static> {
    let mut turn = Turn::replayable(None);
    let mut events = vec![MESSAGE.to_owned()];
    for (index, (id, name, input)) in tools.iter().enumerate() {
        events.push(format!(
            r#"{{"type":"content_block_start","index":{index},"content_block":{{"type":"tool_use","id":"{id}","name":"{name}","input":{{}}}}}}"#
        ));
        events.push(
            serde_json::json!({"type": "content_block_delta", "index": index,
                "delta": {"type": "input_json_delta", "partial_json": input}})
            .to_string(),
        );
        events.push(format!(
            r#"{{"type":"content_block_stop","index":{index}}}"#
        ));
    }
    events.push(END.to_owned());
    for event in events {
        turn.apply(serde_json::from_str(&event).unwrap(), &mut |_| {});
    }
    turn
}

type Answered = (
    Vec<(String, ToolOutput)>,
    Vec<ReplyEvent>,
    Vec<(String, ToolCall)>,
);

fn answered(turn: &Turn<'_>, runs: bool, cancellation: &Cancellation) -> Answered {
    let mut toolbox = FakeToolbox::in_folder("/project");
    let mut hands = Hands {
        toolbox: &mut toolbox,
        permissions: &Answering::with(Decision::Allow),
        cancellation,
    };
    let mut heard = Vec::new();
    let results = answer(turn, runs, &mut hands, &mut |event| heard.push(event));
    (results, heard, toolbox.calls())
}

#[test]
fn runs_each_tool_in_order_and_announces_its_end() {
    let turn = turn_calling(&[
        ("toolu_1", "bash", r#"{"command":"ls"}"#),
        ("toolu_2", "find_files", r#"{"pattern":"*.rs"}"#),
    ]);

    let (results, heard, calls) = answered(&turn, true, &Cancellation::new());

    assert_eq!(calls.len(), 2);
    assert_eq!(results[0].0, "toolu_1");
    assert_eq!(results[0].1.end, ToolEnd::Done(Detail::Exit(0)));
    assert_eq!(results[1].1.end, ToolEnd::Done(Detail::Lines(1)));
    assert_eq!(
        heard,
        [
            ReplyEvent::ToolEnded {
                id: "toolu_1".to_owned(),
                end: ToolEnd::Done(Detail::Exit(0)),
            },
            ReplyEvent::ToolEnded {
                id: "toolu_2".to_owned(),
                end: ToolEnd::Done(Detail::Lines(1)),
            },
        ]
    );
}

#[test]
fn tells_claude_why_a_call_it_wrote_cannot_run() {
    let turn = turn_calling(&[
        ("toolu_1", "rm_everything", "{}"),
        ("toolu_2", "bash", r#"{"command":"#),
    ]);

    let (results, _, calls) = answered(&turn, true, &Cancellation::new());

    assert_eq!(calls, []);
    assert_eq!(results[0].1.end, ToolEnd::Failed);
    assert_eq!(
        results[1].1.content,
        ToolContent::Text(THE_TOOL_INPUT_IS_NOT_JSON.to_owned())
    );
    assert!(results.iter().all(|(_, output)| output.is_error));
}

#[test]
fn answers_without_running_when_the_reply_did_not_ask_or_was_stopped() {
    let turn = turn_calling(&[("toolu_1", "bash", r#"{"command":"ls"}"#)]);
    let stopped = Cancellation::new();
    stopped.cancel();

    let (ended, ended_heard, ended_calls) = answered(&turn, false, &Cancellation::new());
    let (cancelled, _, cancelled_calls) = answered(&turn, true, &stopped);

    assert_eq!(ended_calls, []);
    assert_eq!(cancelled_calls, []);
    assert_eq!(
        ended[0].1.content,
        ToolContent::Text(THE_REPLY_ENDED_BEFORE_THE_TOOL_RAN.to_owned())
    );
    assert_eq!(ended[0].1.end, ToolEnd::Stopped);
    assert!(ended[0].1.is_error);
    assert_eq!(cancelled[0].1, ToolOutput::stopped());
    assert_eq!(
        ended_heard,
        [ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: ToolEnd::Stopped,
        }]
    );
}

#[test]
fn adds_the_cost_of_each_request_and_keeps_the_last_context() {
    let turn = turn_calling(&[]);
    let mut spent = Spent::default();

    spent.add(&turn);
    spent.add(&turn);

    let reply = spent.reply();
    assert_eq!(
        reply.cost.micro_dollars(),
        2 * pricing::cost(turn.model(), turn.usage()).micro_dollars()
    );
    assert_eq!(reply.context_tokens, 14);
    assert_eq!(reply.limits, Limits::default());
}
