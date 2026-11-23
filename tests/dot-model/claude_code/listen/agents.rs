use std::path::Path;

use dot_api::{Detail, ToolKind, ToolUse};
use serde_json::json;

use super::*;
use crate::claude_code::output::AgentCall;

fn heard(line: AgentLine, agents: &mut Agents) -> Vec<ReplyEvent> {
    let mut events = Vec::new();
    agents.hear(line, Some(Path::new("/project")), &mut |event| {
        events.push(event);
    });
    events
}

fn of_agent(event: AgentEvent) -> ReplyEvent {
    ReplyEvent::Agent {
        id: "toolu_1".to_owned(),
        event,
    }
}

#[test]
fn tells_an_agent_that_starts_with_its_kind() {
    let events = heard(
        AgentLine::Started {
            tool: "toolu_1".to_owned(),
            kind: "Explore".to_owned(),
        },
        &mut Agents::default(),
    );

    assert_eq!(
        events,
        [of_agent(AgentEvent::Started {
            kind: "Explore".to_owned()
        })]
    );
}

#[test]
fn tells_what_an_agent_wrote_said_and_called_in_that_order() {
    let message = AgentMessage {
        tool: "toolu_1".to_owned(),
        said: vec!["Looking.".to_owned()],
        calls: vec![AgentCall {
            id: "toolu_9".to_owned(),
            name: "Read".to_owned(),
            input: json!({"file_path": "/project/src/main.rs"}),
        }],
        tokens: 42,
    };

    let events = heard(AgentLine::Message(message), &mut Agents::default());

    assert_eq!(
        events,
        [
            of_agent(AgentEvent::Wrote { tokens: 42 }),
            of_agent(AgentEvent::Said("Looking.".to_owned())),
            of_agent(AgentEvent::ToolStarted {
                id: "toolu_9".to_owned(),
                tool: ToolUse {
                    kind: ToolKind::Read,
                    subject: "src/main.rs".to_owned(),
                },
            }),
        ]
    );
}

#[test]
fn a_message_without_tokens_tells_none() {
    let message = AgentMessage {
        tool: "toolu_1".to_owned(),
        said: Vec::new(),
        calls: Vec::new(),
        tokens: 0,
    };

    assert_eq!(
        heard(AgentLine::Message(message), &mut Agents::default()),
        []
    );
}

#[test]
fn hands_the_report_back_with_the_tokens_once_the_call_ends() {
    let mut agents = Agents::default();
    let kept = heard(
        AgentLine::Reported {
            tool: "toolu_1".to_owned(),
            text: "Two files.".to_owned(),
        },
        &mut agents,
    );

    let reported = agents.handed_back("toolu_1", 243);
    let again = agents.handed_back("toolu_1", 1);

    assert_eq!(kept, []);
    assert_eq!(
        reported,
        of_agent(AgentEvent::Reported {
            text: "Two files.".to_owned(),
            tokens: 243,
        })
    );
    assert_eq!(
        again,
        of_agent(AgentEvent::Reported {
            text: String::new(),
            tokens: 1,
        })
    );
}

fn result(id: &str, is_error: bool, agent_tokens: Option<u64>) -> ToolResult {
    ToolResult {
        id: id.to_owned(),
        is_error,
        detail: Detail::Lines(3),
        agent_tokens,
        change: None,
    }
}

#[test]
fn ends_the_tools_of_claude_refused_failed_or_done() {
    let mut events = Vec::new();

    end_tools(
        vec![
            result("toolu_1", true, None),
            result("toolu_2", true, None),
            result("toolu_3", false, None),
        ],
        (None, &["toolu_1".to_owned()], None),
        &mut Agents::default(),
        &mut |event| events.push(event),
    );

    let ends: Vec<ToolEnd> = events
        .into_iter()
        .map(|event| match event {
            ReplyEvent::ToolEnded { end, .. } => end,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        ends,
        [
            ToolEnd::Refused,
            ToolEnd::Failed,
            ToolEnd::Done(Detail::Lines(3))
        ]
    );
}

#[test]
fn ends_the_tools_of_an_agent_as_the_agent_s() {
    let mut events = Vec::new();

    end_tools(
        vec![result("toolu_9", false, None)],
        (Some("toolu_1".to_owned()), &[], None),
        &mut Agents::default(),
        &mut |event| events.push(event),
    );

    assert_eq!(
        events,
        [of_agent(AgentEvent::ToolEnded {
            id: "toolu_9".to_owned(),
            end: ToolEnd::Done(Detail::Lines(3)),
        })]
    );
}

#[test]
fn hands_an_agent_s_report_back_before_its_call_ends() {
    let mut events = Vec::new();

    end_tools(
        vec![result("toolu_1", false, Some(243))],
        (None, &[], None),
        &mut Agents::default(),
        &mut |event| events.push(event),
    );

    assert_eq!(
        events,
        [
            of_agent(AgentEvent::Reported {
                text: String::new(),
                tokens: 243,
            }),
            ReplyEvent::ToolEnded {
                id: "toolu_1".to_owned(),
                end: ToolEnd::Done(Detail::Lines(3)),
            },
        ]
    );
}

fn changed(id: &str, is_error: bool) -> ToolResult {
    ToolResult {
        change: Some(dot_api::FileChange::between(
            "/p/src/a.rs",
            "a\n",
            "b\n",
            false,
        )),
        ..result(id, is_error, None)
    }
}

#[test]
fn shows_the_change_of_a_tool_that_did_its_work_just_before_its_end() {
    let mut events = Vec::new();

    end_tools(
        vec![
            changed("toolu_1", false),
            changed("toolu_2", false),
            changed("toolu_3", true),
        ],
        (None, &["toolu_2".to_owned()], Some(Path::new("/p"))),
        &mut Agents::default(),
        &mut |event| events.push(event),
    );

    let ReplyEvent::ToolChanged { id, change } = &events[0] else {
        panic!("expected the change first, got {events:?}");
    };
    assert_eq!(id, "toolu_1");
    assert_eq!(change.path, "src/a.rs");
    assert!(matches!(events[1], ReplyEvent::ToolEnded { .. }));
    assert_eq!(events.len(), 4);
}

#[test]
fn shows_no_change_of_an_agent_s_tool() {
    let mut events = Vec::new();

    end_tools(
        vec![changed("toolu_9", false)],
        (Some("toolu_1".to_owned()), &[], None),
        &mut Agents::default(),
        &mut |event| events.push(event),
    );

    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], ReplyEvent::Agent { .. }));
}
