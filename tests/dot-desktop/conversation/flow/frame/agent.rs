use dot_api::AgentEvent;
use dot_api::{ReplyEvent, Text};
use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use super::*;
use crate::conversation::flow::running::tests::with_agents;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, layout, on_the_line, press, scene_at, stage, text_of,
    type_text,
};

fn lane_spot(conversation: &mut Conversation, lane: usize, now: Instant) -> PhysicalPosition<f64> {
    conversation.prepare(&layout(), now, true);
    let stage = stage();
    let first = conversation.below(now).running.unwrap();
    let top = stage.top_row(conversation.reader.focus(now));
    let y = stage.row_top(first + lane, top) + stage.row / 2.0;
    PhysicalPosition::new(f64::from(stage.column[0] + 20.0), f64::from(y))
}

#[test]
fn a_click_on_a_lane_opens_its_agent() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    let spot = lane_spot(&mut conversation, 2, after(now, 100));

    conversation.press(spot, &layout(), after(now, 100));

    assert_eq!(
        conversation
            .agent_view
            .as_ref()
            .map(|view| view.id.as_str()),
        Some("toolu_1")
    );
}

#[test]
fn a_click_while_an_agent_is_shown_opens_no_other_agent() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    let spot = lane_spot(&mut conversation, 2, after(now, 100));
    conversation.open_agent("toolu_2", after(now, 100));

    conversation.press(spot, &layout(), after(now, 200));

    assert_eq!(
        conversation
            .agent_view
            .as_ref()
            .map(|view| view.id.as_str()),
        Some("toolu_2")
    );
}

#[test]
fn a_click_on_the_header_of_the_agents_opens_nothing() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    let spot = lane_spot(&mut conversation, 1, after(now, 100));

    conversation.press(spot, &layout(), after(now, 100));

    assert!(conversation.agent_view.is_none());
}

#[test]
fn opens_no_view_for_an_agent_it_does_not_follow() {
    let now = Instant::now();
    let mut conversation = with_agents(now);

    conversation.open_agent("toolu_9", now);

    assert!(conversation.agent_view.is_none());
}

#[test]
fn shows_the_agent_with_its_line_on_the_reading_line_and_fades_it_in() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", after(now, 100));

    let opening = scene_at(&mut conversation, after(now, 100));
    let open = scene_at(&mut conversation, after(now, 700));

    assert!(opening.animating);
    assert!(
        opening.panes[0]
            .lines
            .iter()
            .all(|placed| placed.line.alpha == 0)
    );
    assert!(!open.animating);
    assert_eq!(flow_texts(&open)[0], "⧉ Explore");
    assert_eq!(on_the_line(&open), ["› agent-view-placeholder"]);
}

#[test]
fn says_in_the_bar_whether_the_agent_works_or_finished() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);

    let working = conversation.activity(&layout(), now, &IdTranslator);
    conversation.hear(
        ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: dot_api::ToolEnd::Done(dot_api::Detail::Nothing),
        },
        after(now, 500),
        &IdTranslator,
    );
    let done = conversation.activity(&layout(), after(now, 600), &IdTranslator);

    assert_eq!(working.text.as_deref(), Some("state-agent-working Explore"));
    assert!(working.live);
    assert_eq!(done.text.as_deref(), Some("state-agent-done Explore"));
    assert!(!done.live);
}

#[test]
fn escape_goes_back_to_the_conversation_and_fades_it_back_in() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);

    let command = press(&mut conversation, NamedKey::Escape, after(now, 1_000));
    let back = scene_at(&mut conversation, after(now, 1_000));
    let settled = scene_at(&mut conversation, after(now, 1_600));

    assert_eq!(command, None);
    assert!(conversation.agent_view.is_none());
    assert!(back.animating);
    assert_ne!(settled.panes[0].lines, []);
}

#[test]
fn nothing_typed_reaches_the_agent_nor_the_draft() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);

    type_text(&mut conversation, "faster", after(now, 100));
    let sent = press(&mut conversation, NamedKey::Enter, after(now, 200));

    assert_eq!(sent, None);
    assert!(conversation.prompter.is_empty());
    assert!(conversation.agent_view.is_some());
}

#[test]
fn the_wheel_scrolls_the_view_and_not_the_conversation() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    for index in 0..10 {
        conversation.hear(
            ReplyEvent::Agent {
                id: "toolu_1".to_owned(),
                event: AgentEvent::Said(format!("step {index}")),
            },
            now,
            &IdTranslator,
        );
    }
    conversation.open_agent("toolu_1", now);
    let at = PhysicalPosition::new(100.0, 100.0);

    conversation.scroll(
        crate::event::Wheel::Lines(1.0),
        (at, &layout()),
        now,
        &IdTranslator,
    );
    let scrolled = conversation.agent_view.as_ref().unwrap().scrolled();
    conversation.scroll(
        crate::event::Wheel::Lines(-50.0),
        (at, &layout()),
        now,
        &IdTranslator,
    );

    assert!((scrolled - 3.0).abs() < f64::EPSILON);
    assert!(conversation.agent_view.as_ref().unwrap().scrolled().abs() < f64::EPSILON);
    assert!(!conversation.reader.is_reviewing());
}

#[test]
fn keeps_the_instrument_beside_an_agent_s_view() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);
    let later = after(now, 1_000);

    let scene = scene_at(&mut conversation, later);

    let instrument = scene.panes.iter().any(|pane| {
        pane.lines
            .iter()
            .any(|placed| text_of(placed).contains(Text::InstrumentTokens.id()))
    });
    assert!(instrument, "{:?}", scene.panes.len());
}

#[test]
fn leaving_the_session_closes_the_view_of_an_agent() {
    let now = Instant::now();
    let mut conversation = with_agents(now);
    conversation.open_agent("toolu_1", now);

    conversation.leave();

    assert!(conversation.agent_view.is_none());
    assert!(conversation.back_from_agent.is_none());
}
