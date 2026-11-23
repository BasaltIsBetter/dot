use dot_api::{ReplyEvent, ToolKind, ToolUse};

use super::*;
use crate::conversation::flow::tests::{IdTranslator, NARROW, RETINA, after, layout, sent};

fn activity_of(conversation: &Conversation, now: Instant) -> Activity {
    conversation.activity(&layout(), now, &IdTranslator)
}

fn start(conversation: &mut Conversation, id: &str, kind: ToolKind, subject: &str, now: Instant) {
    let tool = ToolUse {
        kind,
        subject: subject.to_owned(),
    };
    let started = ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool,
    };
    conversation.hear(started, now, &IdTranslator);
}

#[test]
fn says_nothing_while_claude_does_not_reply() {
    let conversation = Conversation::default();

    assert_eq!(
        activity_of(&conversation, Instant::now()),
        Activity::default()
    );
}

#[test]
fn claude_thinks_until_its_answer_arrives_then_writes() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    let thinking = activity_of(&conversation, now);
    conversation.hear(ReplyEvent::Thinking("hmm".into()), now, &IdTranslator);
    let still_thinking = activity_of(&conversation, now);

    conversation.hear(ReplyEvent::Text("Hi".into()), after(now, 10), &IdTranslator);

    assert_eq!(thinking.text.as_deref(), Some("state-thinking"));
    assert!(thinking.live);
    assert_eq!(still_thinking.text.as_deref(), Some("state-thinking"));
    assert_eq!(
        activity_of(&conversation, after(now, 10)).text.as_deref(),
        Some("state-writing")
    );
}

#[test]
fn names_the_last_tool_still_running_with_its_verb() {
    let now = Instant::now();
    let mut conversation = sent("look", now);
    start(
        &mut conversation,
        "t1",
        ToolKind::Read,
        "src/layout.rs",
        now,
    );
    start(&mut conversation, "t2", ToolKind::Run, "cargo test", now);

    let running = activity_of(&conversation, now);
    conversation.hear(
        ReplyEvent::ToolEnded {
            id: "t2".into(),
            end: dot_api::ToolEnd::Done(dot_api::Detail::Nothing),
        },
        after(now, 5),
        &IdTranslator,
    );

    assert_eq!(running.text.as_deref(), Some("state-running cargo test"));
    assert_eq!(
        activity_of(&conversation, after(now, 5)).text.as_deref(),
        Some("state-reading layout.rs")
    );
}

#[test]
fn counts_the_agents_claude_waits_for_before_any_other_tool() {
    let now = Instant::now();
    let mut conversation = sent("explore", now);
    start(
        &mut conversation,
        "a1",
        ToolKind::Delegate,
        "explore the canvas",
        now,
    );
    start(&mut conversation, "t1", ToolKind::Read, "a.rs", now);

    assert_eq!(
        activity_of(&conversation, now).text.as_deref(),
        Some("state-waiting-for-agents")
    );
}

#[test]
fn names_each_kind_of_tool_with_its_own_verb() {
    for (glyph, verb) in [
        (Glyph::Read, Text::StateReading),
        (Glyph::Search, Text::StateSearching),
        (Glyph::Edit, Text::StateEditing),
        (Glyph::Create, Text::StateCreating),
        (Glyph::Command, Text::StateRunning),
        (Glyph::Web, Text::StateBrowsing),
        (Glyph::Agents, Text::StateUsing),
        (Glyph::Other, Text::StateUsing),
    ] {
        assert_eq!(verb_of(glyph), verb, "{glyph:?}");
    }
}

#[test]
fn adds_the_measures_when_no_instrument_has_room_beside_the_line() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    conversation.hear(
        ReplyEvent::Text("Hi there".into()),
        after(now, 100),
        &IdTranslator,
    );
    let narrow = Layout::reflowed(NARROW, RETINA);

    let wide = activity_of(&conversation, after(now, 300));
    let squeezed = conversation.activity(&narrow, after(now, 300), &IdTranslator);

    assert_eq!(wide.text.as_deref(), Some("state-writing"));
    let squeezed = squeezed.text.unwrap();
    assert!(squeezed.starts_with("state-writing · "), "{squeezed}");
    assert!(squeezed.contains("unit-tokens-per-second"), "{squeezed}");
    assert!(squeezed.contains("unit-tokens "), "{squeezed}");
}
