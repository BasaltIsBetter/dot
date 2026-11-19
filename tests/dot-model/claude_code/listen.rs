use dot_api::{Cancellation, Decision, Detail};

use super::*;
use crate::tool::tests::{Answering, FakeToolbox};

const ASKED: &str = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_1"}}"#;
const ENDED: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","is_error":true}]}}"#;
const FAILED: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_2","is_error":true}]}}"#;
const DONE: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_3"}]},"tool_use_result":{"numFiles":2}}"#;

fn heard(lines: &[&str], decision: Decision) -> (Vec<ReplyEvent>, Vec<dot_api::Permission>) {
    let permissions = Answering::with(decision);
    let mut session = None;
    let mut stdin = None;
    let mut heard = Vec::new();
    let hands = Hands {
        toolbox: &FakeToolbox::without_folder(),
        permissions: &permissions,
        cancellation: &Cancellation::new(),
    };
    Listener {
        session: &mut session,
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |event| heard.push(event),
    }
    .listen(lines.join("\n").as_bytes())
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
