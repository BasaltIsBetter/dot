use dot_api::{Action, Detail, ToolEnd, ToolKind, ToolUse};
use winit::keyboard::NamedKey;

use super::*;
use crate::conversation::Intent;
use crate::conversation::flow::tests::{
    IdTranslator, after, compact, layout, over_prompter, over_transcript, pane_texts, press_return,
    scene_at, type_text,
};
use crate::conversation::prompter::Edit;
use crate::event::Wheel;

fn replying(now: Instant) -> Conversation {
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "hello");
    press_return(&mut conversation, now);
    conversation
}

fn started(id: &str, command: &str) -> ReplyEvent {
    ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool: ToolUse {
            kind: ToolKind::Run,
            subject: command.to_owned(),
        },
    }
}

fn ended(id: &str, end: ToolEnd) -> ReplyEvent {
    ReplyEvent::ToolEnded {
        id: id.to_owned(),
        end,
    }
}

fn run(command: &str) -> Permission {
    Permission {
        tool_id: "t1".to_owned(),
        action: Action::Run {
            command: command.to_owned(),
        },
    }
}

fn hear(conversation: &mut Conversation, event: ReplyEvent, now: Instant) {
    conversation.hear(event, &layout(), now, &IdTranslator);
}

fn asking(now: Instant, command: &str) -> Conversation {
    let mut conversation = replying(now);
    conversation.ask(&run(command), &layout(), now, &IdTranslator);
    conversation
}

#[test]
fn shows_each_tool_then_how_it_ended() {
    let now = Instant::now();
    let mut conversation = replying(now);

    hear(&mut conversation, started("t1", "cargo test"), now);
    let running = pane_texts(&scene_at(&conversation, after(now, 2_000)), 0);
    hear(
        &mut conversation,
        ended("t1", ToolEnd::Done(Detail::Exit(0))),
        now,
    );
    hear(&mut conversation, ReplyEvent::Text("Green.".into()), now);
    let done = pane_texts(&scene_at(&conversation, after(now, 2_000)), 0);

    assert!(
        running.contains(&"tool-run cargo test_".to_owned()),
        "{running:?}"
    );
    let tool = done
        .iter()
        .position(|row| row == "tool-run cargo test  tool-exit 0");
    assert_eq!(
        tool.map(|row| done[row + 1].as_str()),
        Some("Green._"),
        "{done:?}"
    );
}

#[test]
fn asks_in_place_of_the_prompter_and_gives_it_back() {
    let now = Instant::now();
    let mut conversation = replying(now);
    type_text(&mut conversation, "next");
    let before = scene_at(&conversation, now);

    conversation.ask(&run("rm -r build"), &layout(), now, &IdTranslator);
    assert!(conversation.is_asking());
    let asked = scene_at(&conversation, now);
    conversation.close_question(&layout());
    let after_answer = scene_at(&conversation, now);

    assert!(!conversation.is_asking());
    assert_eq!(
        pane_texts(&asked, 1),
        vec![
            "› permission-run",
            "rm -r build",
            "permission-allow   permission-deny   grant-command",
        ]
    );
    assert!(asked.panes[1].top < before.panes[1].top);
    assert!(asked.panes[0].bottom < before.panes[0].bottom);
    assert!(asked.panes[1].shift.abs() < f32::EPSILON);
    assert_eq!(pane_texts(&after_answer, 1), pane_texts(&before, 1));
    assert!((after_answer.panes[1].top - before.panes[1].top).abs() < f32::EPSILON);
}

#[test]
fn keeps_the_draft_as_it_is_while_asking() {
    let now = Instant::now();
    let mut conversation = replying(now);
    let before = pane_texts(&scene_at(&conversation, now), 1);
    conversation.ask(&run("ls"), &layout(), now, &IdTranslator);

    type_text(&mut conversation, "typed");
    conversation.compose("^".to_owned());
    conversation.commit("é".to_owned(), &layout());
    conversation.type_key(
        Intent::Edit(Edit::Insert("x".to_owned())),
        &layout(),
        now,
        || None,
    );
    conversation.close_question(&layout());

    assert_eq!(pane_texts(&scene_at(&conversation, now), 1), before);
}

#[test]
fn answers_only_while_asking() {
    let now = Instant::now();
    let ready = after(now, 300);
    let enter = Key::Named(NamedKey::Enter);
    let mut conversation = asking(now, "ls");

    let answer = conversation.answer_to(&enter, ModifiersState::empty(), ready);
    conversation.close_question(&layout());

    assert_eq!(answer, Some(Answer::Allow));
    assert_eq!(
        conversation.answer_to(&enter, ModifiersState::empty(), ready),
        None
    );
}

#[test]
fn asks_nothing_outside_a_reply() {
    let now = Instant::now();
    let mut conversation = Conversation::default();

    conversation.ask(&run("ls"), &layout(), now, &IdTranslator);

    assert!(!conversation.is_asking());
}

#[test]
fn the_end_of_the_reply_takes_the_question_away() {
    let now = Instant::now();
    let mut conversation = asking(now, "ls");

    conversation.end_reply(Some("reply-stopped"), &layout(), now);

    assert!(!conversation.is_asking());
    assert_eq!(pane_texts(&scene_at(&conversation, now), 1)[0], "› ");
}

#[test]
fn the_wheel_over_the_prompter_scrolls_a_long_question() {
    let now = Instant::now();
    let lines: Vec<String> = (0..30).map(|line| format!("echo {line}")).collect();
    let mut conversation = asking(now, &lines.join("\n"));

    let first = pane_texts(&scene_at(&conversation, now), 1);
    conversation.scroll(Wheel::Lines(-20.0), over_prompter(), &layout(), now);
    let last = pane_texts(&scene_at(&conversation, now), 1);

    assert_eq!(first[0], "› permission-run");
    assert_eq!(first.len(), 8);
    assert_eq!(
        last.last().map(String::as_str),
        Some("permission-allow   permission-deny   grant-command")
    );
}

#[test]
fn a_click_on_the_question_leaves_the_caret_alone() {
    let now = Instant::now();
    let mut conversation = replying(now);
    type_text(&mut conversation, "hello");
    let caret = conversation.caret_area(&layout());
    conversation.ask(&run("ls"), &layout(), now, &IdTranslator);

    conversation.press(over_prompter(), &layout());
    conversation.close_question(&layout());

    assert_eq!(conversation.caret_area(&layout()), caret);
}

#[test]
fn the_question_follows_a_narrower_column() {
    let now = Instant::now();
    let mut conversation = asking(now, &"word ".repeat(60));
    let rows = |conversation: &Conversation| conversation.question.as_ref().map(Question::rows);
    let wide = rows(&conversation);

    conversation.prepare(&compact(), now, &IdTranslator);

    assert_eq!(wide, Some(2 + 4));
    assert_eq!(rows(&conversation), Some(2 + 5));
}

#[test]
fn a_closing_question_hands_the_end_back_to_the_transcript() {
    let now = Instant::now();
    let settled = after(now, 2_000);
    let mut conversation = replying(now);
    let long = (0..60)
        .map(|row| format!("row {row}\n"))
        .collect::<Vec<_>>()
        .concat();
    hear(&mut conversation, ReplyEvent::Text(long), now);
    conversation.ask(&run("ls"), &layout(), settled, &IdTranslator);
    let one_row = f64::from(layout().line_height());
    conversation.scroll(
        Wheel::Pixels(one_row),
        over_transcript(),
        &layout(),
        settled,
    );

    conversation.close_question(&layout());
    hear(
        &mut conversation,
        ReplyEvent::Text("more\nrows\nlast".into()),
        settled,
    );

    let shown = pane_texts(&scene_at(&conversation, after(now, 4_000)), 0);
    assert_eq!(shown.last().map(String::as_str), Some("last_"));
}
