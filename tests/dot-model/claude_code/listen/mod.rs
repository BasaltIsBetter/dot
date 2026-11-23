use std::sync::mpsc;

use dot_api::{AgentEvent, Cancellation, Decision, Detail, ToolEnd};

use super::*;
use crate::claude_code::listen::channel::read_lines;
use crate::tool::tests::{Answering, FakeToolbox, Untouched};

const ASKED: &str = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_1"}}"#;
const ENDED: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","is_error":true}]}}"#;
const FAILED: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_2","is_error":true}]}}"#;
const DONE: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_3"}]},"tool_use_result":{"numFiles":2}}"#;

pub(crate) fn arrivals(bytes: &[u8]) -> Receiver<Incoming> {
    let (sender, receiver) = mpsc::channel();
    read_lines(bytes, &sender);
    receiver
}

fn heard(lines: &[&str], decision: Decision) -> (Vec<ReplyEvent>, Vec<dot_api::Permission>) {
    let permissions = Answering::with(decision);
    let mut session = None;
    let mut stdin = None;
    let mut heard = Vec::new();
    let hands = Hands {
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };
    Listener {
        session: &mut session,
        ledger: &mut Ledger::NOTHING_YET.clone(),
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |event| heard.push(event),
    }
    .listen(&arrivals(lines.join("\n").as_bytes()))
    .unwrap();
    (heard, permissions.asked())
}

#[test]
fn ends_a_refused_tool_as_refused_and_the_others_by_their_result() {
    let (events, asked) = heard(&[ASKED, ENDED, FAILED, DONE], Decision::Deny);

    assert_eq!(asked.len(), 1);
    assert_eq!(
        events,
        [
            ReplyEvent::ToolEnded {
                id: "toolu_1".to_owned(),
                end: ToolEnd::Refused,
            },
            ReplyEvent::ToolEnded {
                id: "toolu_2".to_owned(),
                end: ToolEnd::Failed,
            },
            ReplyEvent::ToolEnded {
                id: "toolu_3".to_owned(),
                end: ToolEnd::Done(Detail::Files(2)),
            },
        ]
    );
}

#[test]
fn an_allowed_tool_that_fails_shows_as_failed() {
    let (events, _) = heard(&[ASKED, ENDED], Decision::Allow);

    assert_eq!(
        events,
        [ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: ToolEnd::Failed,
        }]
    );
}

#[test]
fn stops_at_the_result_line() {
    let result = r#"{"type":"result","is_error":false,"total_cost_usd":0}"#;

    let (events, asked) = heard(&[result, ASKED, DONE], Decision::Allow);

    assert_eq!(events, []);
    assert_eq!(asked, []);
}

const SESSION: &str = "3898f4c9-b1b3-480e-b180-b139f7eb2054";
const SUMMARIZED: &str = r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"auto","pre_tokens":154908,"post_tokens":16764}}"#;

fn request_reading(input: u64, cached: u64, written: u64) -> String {
    format!(
        r#"{{"type":"stream_event","event":{{"type":"message_start","message":{{"model":"claude-opus-5-5","usage":{{"input_tokens":{input},"cache_read_input_tokens":{cached},"cache_creation_input_tokens":{written},"output_tokens":1}}}}}}}}"#
    )
}

fn base_heard(lines: &[&str], mut session: Option<SessionId>) -> Option<u64> {
    let permissions = Answering::with(Decision::Allow);
    let mut stdin = None;
    let hands = Hands {
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };
    let heard = Listener {
        session: &mut session,
        ledger: &mut Ledger::NOTHING_YET.clone(),
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |_| {},
    }
    .listen(&arrivals(lines.join("\n").as_bytes()))
    .unwrap();
    assert_eq!(heard.spent().base_tokens, heard.base_tokens);
    heard.base_tokens
}

#[test]
fn tells_where_claude_code_summarized_the_conversation() {
    let (events, _) = heard(&[SUMMARIZED], Decision::Allow);

    assert_eq!(
        events,
        [ReplyEvent::Compacted {
            before: 154_908,
            after: 16_764,
        }]
    );
}

#[test]
fn the_first_request_of_a_new_conversation_gives_its_base() {
    let first = request_reading(2, 649, 1_680);
    let later = request_reading(2, 2_331, 900);

    assert_eq!(base_heard(&[&first, &later], None), Some(2 + 649 + 1_680));
}

#[test]
fn a_conversation_that_goes_on_gives_no_base() {
    let request = request_reading(2, 649, 1_680);

    assert_eq!(base_heard(&[&request], SessionId::parse(SESSION)), None);
}

#[test]
fn a_request_claude_never_read_gives_no_base() {
    let unread = request_reading(0, 0, 0);
    let read = request_reading(0, 30, 0);

    assert_eq!(base_heard(&[&unread], None), None);
    assert_eq!(base_heard(&[&unread, &read], None), Some(30));
    assert_eq!(base_heard(&[SUMMARIZED], None), None);
}
fn ledger_after(lines: &[&str]) -> Ledger {
    let permissions = Answering::with(Decision::Allow);
    let (mut session, mut stdin) = (None, None);
    let mut ledger = Ledger::NOTHING_YET;
    let hands = Hands {
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };
    Listener {
        session: &mut session,
        ledger: &mut ledger,
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |_| {},
    }
    .listen(&arrivals(lines.join("\n").as_bytes()))
    .unwrap();
    ledger
}

#[test]
fn counts_each_request_and_each_summary_for_the_next_window() {
    let first = request_reading(2, 649, 1_680);
    let after_a_summary = request_reading(2, 100, 900);
    let text = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}}"#;

    let heard = ledger_after(&[&first, text, SUMMARIZED, &after_a_summary]);

    let mut counted = Ledger::NOTHING_YET;
    counted.request(Request {
        read: 2 + 649 + 1_680,
        written: 1_680,
        written_for_an_hour: 0,
    });
    counted.summarized();
    counted.request(Request {
        read: 2 + 100 + 900,
        written: 900,
        written_for_an_hour: 0,
    });
    assert_eq!(heard, counted);
}

const AGENT_STARTED: &str = r#"{"type":"system","subtype":"task_started","task_id":"ace55","tool_use_id":"toolu_01Ue","description":"List files in current folder","subagent_type":"general-purpose"}"#;
const AGENT_PROMPT: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Run ls"}]},"parent_tool_use_id":"toolu_01Ue"}"#;
const AGENT_CALLS: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_01Fu","name":"Bash","input":{"command":"ls"}}],"usage":{"output_tokens":17}},"parent_tool_use_id":"toolu_01Ue"}"#;
const AGENT_RAN: &str = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Fu","type":"tool_result","content":"a.txt","is_error":false}]},"parent_tool_use_id":"toolu_01Ue","tool_use_result":{"stdout":"a.txt"}}"#;
const AGENT_REPORTED: &str = r#"{"type":"system","subtype":"task_notification","task_id":"ace55","tool_use_id":"toolu_01Ue","status":"completed","summary":"One file: a.txt"}"#;
const AGENT_HANDED_BACK: &str = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Ue","type":"tool_result","content":[{"type":"text","text":"One file: a.txt"}]}]},"parent_tool_use_id":null,"tool_use_result":{"status":"completed","agentType":"general-purpose","usage":{"output_tokens":243}}}"#;

#[test]
fn follows_an_agent_from_its_start_to_its_report() {
    let (events, _) = heard(
        &[
            AGENT_STARTED,
            AGENT_PROMPT,
            AGENT_CALLS,
            AGENT_RAN,
            AGENT_REPORTED,
            AGENT_HANDED_BACK,
        ],
        Decision::Allow,
    );

    let of_agent = |event| ReplyEvent::Agent {
        id: "toolu_01Ue".to_owned(),
        event,
    };
    assert_eq!(
        events,
        [
            of_agent(AgentEvent::Started {
                kind: "general-purpose".to_owned()
            }),
            of_agent(AgentEvent::Wrote { tokens: 17 }),
            of_agent(AgentEvent::ToolStarted {
                id: "toolu_01Fu".to_owned(),
                tool: dot_api::ToolUse {
                    kind: dot_api::ToolKind::Run,
                    subject: "ls".to_owned(),
                },
            }),
            of_agent(AgentEvent::ToolEnded {
                id: "toolu_01Fu".to_owned(),
                end: ToolEnd::Done(Detail::Nothing),
            }),
            of_agent(AgentEvent::Reported {
                text: "One file: a.txt".to_owned(),
                tokens: 243,
            }),
            ReplyEvent::ToolEnded {
                id: "toolu_01Ue".to_owned(),
                end: ToolEnd::Done(Detail::Nothing),
            },
        ]
    );
}

#[test]
fn hands_live_pieces_over_among_the_lines_in_the_order_they_came() {
    let start = r#"{"type":"stream_event","event":{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{}}}}"#;
    let tool = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"Bash","input":{}}}}"#;
    let input = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"ls\"}"}}}"#;
    let stop = r#"{"type":"stream_event","event":{"type":"content_block_stop","index":0}}"#;
    let (sender, receiver) = mpsc::channel();
    for line in [start, tool, input, stop] {
        sender
            .send(Incoming::Line(line.as_bytes().to_vec()))
            .unwrap();
    }
    sender
        .send(Incoming::Piece {
            connection: 0,
            command: "ls".to_owned(),
            bytes: b"a.txt\n".to_vec(),
        })
        .unwrap();
    sender.send(Incoming::Ended(Ok(()))).unwrap();
    let permissions = Answering::with(Decision::Allow);
    let mut session = None;
    let mut stdin = None;
    let mut events = Vec::new();
    let hands = Hands {
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };

    Listener {
        session: &mut session,
        ledger: &mut Ledger::NOTHING_YET.clone(),
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |event| events.push(event),
    }
    .listen(&receiver)
    .unwrap();

    assert_eq!(
        events.last(),
        Some(&ReplyEvent::ToolOutput {
            id: "toolu_1".to_owned(),
            lines: vec!["a.txt".to_owned()],
        })
    );
}

#[test]
fn hands_back_the_error_that_cut_the_stream() {
    let (sender, receiver) = mpsc::channel();
    sender
        .send(Incoming::Ended(Err(io::ErrorKind::BrokenPipe.into())))
        .unwrap();
    let permissions = Answering::with(Decision::Allow);
    let (mut session, mut stdin) = (None, None);
    let hands = Hands {
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };

    let listened = Listener {
        session: &mut session,
        ledger: &mut Ledger::NOTHING_YET.clone(),
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |_| {},
    }
    .listen(&receiver);

    assert!(matches!(listened, Err(error) if error.kind() == io::ErrorKind::BrokenPipe));
}
