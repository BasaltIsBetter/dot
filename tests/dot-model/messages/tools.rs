use dot_api::{Cancellation, Decision, Detail, TextSearch, ToolCall};

use super::*;
use crate::stream::Turn;
use crate::tool::tests::{Answering, FakeToolbox, Naming, Untouched};

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
        compactor: &Untouched,
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
            ReplyEvent::ToolOutput {
                id: "toolu_1".to_owned(),
                lines: vec!["\u{1b}[1mrunning\u{1b}[0m ls".to_owned()],
            },
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
fn hands_over_what_a_running_command_prints_as_a_person_reads_it() {
    let turn = turn_calling(&[("toolu_1", "bash", r#"{"command":"cargo test"}"#)]);
    let mut toolbox = FakeToolbox::in_folder("/project");
    let mut hands = Hands {
        toolbox: &mut toolbox,
        permissions: &Answering::with(Decision::Allow),
        cancellation: &Cancellation::new(),
        compactor: &Naming,
    };
    let mut heard = Vec::new();

    answer(&turn, true, &mut hands, &mut |event| heard.push(event));

    assert_eq!(
        heard[0],
        ReplyEvent::ToolOutput {
            id: "toolu_1".to_owned(),
            lines: vec!["[1mrunning[0m cargo test".to_owned()],
        }
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
    assert_eq!(reply.output_tokens, 2 * turn.output_tokens());
    assert!(turn.output_tokens() > 0);
    assert_eq!(reply.limits, Limits::default());
}

fn turn_that_read(input_tokens: u64) -> Turn<'static> {
    let start = format!(
        r#"{{"type":"message_start","message":{{"model":"claude-opus-5-5","usage":{{"input_tokens":{input_tokens},"output_tokens":1}}}}}}"#
    );
    let mut turn = Turn::replayable(None);
    turn.apply(serde_json::from_str(&start).unwrap(), &mut |_| {});
    turn
}

#[test]
fn the_first_request_claude_read_is_the_base_of_a_new_conversation() {
    let mut spent = Spent::of_a_reply(true);

    spent.add(&turn_that_read(0));
    assert_eq!(spent.reply().base_tokens, None);
    spent.add(&turn_that_read(7));
    spent.add(&turn_that_read(90));

    assert_eq!(spent.reply().base_tokens, Some(7));
    assert_eq!(spent.reply().context_tokens, 91);
}

#[test]
fn a_reply_that_carries_a_conversation_on_has_no_base() {
    let mut spent = Spent::of_a_reply(false);

    spent.add(&turn_that_read(7));

    assert_eq!(spent.reply().base_tokens, None);
    assert_eq!(Spent::default().reply().base_tokens, None);
}

fn search(show: SearchShow) -> ToolCall {
    ToolCall::SearchText(TextSearch {
        pattern: "fn".to_owned(),
        folder: None,
        files: None,
        ignore_case: false,
        show,
    })
}

fn find() -> ToolCall {
    ToolCall::FindFiles {
        pattern: "*.rs".to_owned(),
        folder: None,
    }
}

fn run() -> ToolCall {
    ToolCall::Run {
        command: "ls".to_owned(),
    }
}

fn text_of(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => "an image",
    }
}

#[test]
fn shortens_what_a_command_that_ran_and_a_search_return() {
    let cases = [
        (run(), Detail::Exit(0), "Command(\"ls\") of 6 bytes"),
        (run(), Detail::Exit(101), "Command(\"ls\") of 6 bytes"),
        (find(), Detail::Files(2), "Paths of 6 bytes"),
        (
            search(SearchShow::Lines),
            Detail::Matches(2),
            "Matches of 6 bytes",
        ),
        (
            search(SearchShow::Files),
            Detail::Files(2),
            "Paths of 6 bytes",
        ),
    ];

    for (call, detail, expected) in cases {
        let shorter = shortened(&call, ToolOutput::done("output", detail), &Naming);

        assert_eq!(text_of(&shorter), expected);
        assert_eq!(shorter.end, ToolEnd::Done(detail));
        assert!(!shorter.is_error);
    }
}

#[test]
fn leaves_alone_what_did_not_run_counts_and_the_contents_of_files() {
    let view = ToolCall::View {
        path: "src/lib.rs".to_owned(),
        lines: None,
    };
    let cases = [
        (run(), ToolOutput::failed("output")),
        (run(), ToolOutput::stopped()),
        (find(), ToolOutput::failed("output")),
        (search(SearchShow::Lines), ToolOutput::refused()),
        (
            search(SearchShow::Counts),
            ToolOutput::done("output", Detail::Matches(2)),
        ),
        (view, ToolOutput::done("output", Detail::Lines(1))),
        (
            ToolCall::RestartShell,
            ToolOutput::done("output", Detail::Nothing),
        ),
    ];

    for (call, output) in cases {
        assert_eq!(
            shortened(&call, output.clone(), &Naming),
            output,
            "{call:?}"
        );
    }
}

#[test]
fn keeps_the_output_the_compactor_cannot_shorten() {
    let output = ToolOutput::done("output", Detail::Exit(0));

    assert_eq!(shortened(&run(), output.clone(), &Untouched), output);
}

#[test]
fn shows_the_change_a_tool_made_just_before_its_end() {
    let turn = turn_calling(&[(
        "toolu_1",
        "str_replace_based_edit_tool",
        r#"{"command":"create","path":"/project/src/new.rs","file_text":"a\n"}"#,
    )]);

    let (results, heard, _) = answered(&turn, true, &Cancellation::new());

    assert_eq!(results[0].1.change, None);
    let [
        ReplyEvent::ToolChanged { id, change },
        ReplyEvent::ToolEnded { .. },
    ] = heard.as_slice()
    else {
        panic!("expected the change then the end, got {heard:?}");
    };
    assert_eq!(id, "toolu_1");
    assert_eq!(change.path, "src/new.rs");
    assert!(change.created);
}
