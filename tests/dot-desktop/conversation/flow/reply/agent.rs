use dot_api::{AgentEvent, Detail, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::conversation::flow::tests::{IdTranslator, after, flow_texts, scene_at, sent};

fn agent_event(event: AgentEvent) -> ReplyEvent {
    ReplyEvent::Agent {
        id: "toolu_1".to_owned(),
        event,
    }
}

fn hand_over(conversation: &mut Conversation, now: Instant) {
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: "toolu_1".to_owned(),
            tool: ToolUse {
                kind: ToolKind::Delegate,
                subject: "find the grid".to_owned(),
            },
        },
        now,
        &IdTranslator,
    );
}

fn working(now: Instant) -> Conversation {
    let mut conversation = sent("where is the grid?", now);
    hand_over(&mut conversation, now);
    conversation.hear(
        agent_event(AgentEvent::Started {
            kind: "Explore".to_owned(),
        }),
        now,
        &IdTranslator,
    );
    conversation
}

fn read(conversation: &mut Conversation, id: &str, path: &str, now: Instant) {
    conversation.hear(
        agent_event(AgentEvent::ToolStarted {
            id: id.to_owned(),
            tool: ToolUse {
                kind: ToolKind::Read,
                subject: path.to_owned(),
            },
        }),
        now,
        &IdTranslator,
    );
}

#[test]
fn follows_an_agent_claude_hands_a_task_to() {
    let now = Instant::now();
    let mut conversation = working(now);

    read(&mut conversation, "a", "src/layout.rs", now);
    conversation.hear(
        agent_event(AgentEvent::Said("Found it.".to_owned())),
        now,
        &IdTranslator,
    );

    let agent = conversation.agents.get("toolu_1").unwrap();
    assert_eq!(agent.kind, "Explore");
    assert_eq!(agent.mission, "find the grid");
    assert_eq!(agent.said, ["Found it."]);
    assert_eq!(agent.current().unwrap().tool.label, "layout.rs");
}

#[test]
fn ends_an_agent_s_tool_with_its_result() {
    let now = Instant::now();
    let mut conversation = working(now);
    read(&mut conversation, "a", "src/layout.rs", now);

    conversation.hear(
        agent_event(AgentEvent::ToolEnded {
            id: "a".to_owned(),
            end: ToolEnd::Done(Detail::Lines(212)),
        }),
        now,
        &IdTranslator,
    );

    let action = conversation.agents.get("toolu_1").unwrap().actions[0].clone();
    assert_eq!(action.result.as_deref(), Some("212 gutter-lines"));
}

#[test]
fn counts_what_agents_write_in_the_instrument() {
    let now = Instant::now();
    let mut conversation = working(now);

    conversation.hear(
        agent_event(AgentEvent::Wrote { tokens: 17 }),
        after(now, 100),
        &IdTranslator,
    );
    conversation.hear(
        agent_event(AgentEvent::Reported {
            text: "Two files.".to_owned(),
            tokens: 243,
        }),
        after(now, 900),
        &IdTranslator,
    );

    let measures = conversation.instrument.current(after(now, 1_000)).unwrap();
    assert_eq!(measures.tokens, 243);
    assert_eq!(
        conversation
            .agents
            .get("toolu_1")
            .unwrap()
            .report
            .as_deref(),
        Some("Two files.")
    );
}

#[test]
fn sums_a_finished_agent_up_in_the_reply() {
    let now = Instant::now();
    let mut conversation = working(now);
    read(&mut conversation, "a", "src/layout.rs", now);
    read(&mut conversation, "b", "src/view.rs", now);

    conversation.hear(
        ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: ToolEnd::Done(Detail::Nothing),
        },
        after(now, 7_800),
        &IdTranslator,
    );

    let texts = flow_texts(&scene_at(&mut conversation, after(now, 8_000)));
    assert!(
        texts
            .iter()
            .any(|text| text == "⧉ Explore · agent-read · 7decimal-separator8 unit-seconds"),
        "{texts:?}"
    );
    assert_eq!(conversation.agents.working().count(), 0);
}

#[test]
fn names_an_agent_by_its_task_until_it_says_its_kind() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    hand_over(&mut conversation, now);

    let agent = conversation.agents.get("toolu_1").unwrap();
    let summed = summary(agent, after(now, 1_000), &IdTranslator);

    assert_eq!(name_of(agent), "find the grid");
    assert_eq!(summed, "find the grid · 1decimal-separator0 unit-seconds");
}

#[test]
fn follows_no_agent_outside_a_reply() {
    let now = Instant::now();
    let mut conversation = Conversation::default();

    hand_over(&mut conversation, now);

    assert!(conversation.agents.get("toolu_1").is_none());
}

#[test]
fn keeps_what_a_running_command_printed() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: "toolu_2".to_owned(),
            tool: ToolUse {
                kind: ToolKind::Run,
                subject: "cargo test".to_owned(),
            },
        },
        now,
        &IdTranslator,
    );

    conversation.hear(
        ReplyEvent::ToolOutput {
            id: "toolu_2".to_owned(),
            lines: vec!["Compiling dot".to_owned()],
        },
        after(now, 100),
        &IdTranslator,
    );

    let running: Vec<_> = conversation.transcript.running().collect();
    assert_eq!(running[0].output, ["Compiling dot"]);
}
