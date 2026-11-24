use dot_api::{Action, Detail, ToolEnd, ToolKind, ToolUse};
use winit::keyboard::NamedKey;

use super::*;
use crate::conversation::Intent;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, press_return, scene_at, sent, text_of, type_text,
};
use crate::conversation::prompter::Edit;

fn started(id: &str, kind: ToolKind, subject: &str) -> ReplyEvent {
    ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool: ToolUse {
            kind,
            subject: subject.to_owned(),
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
    conversation.hear(event, now, &IdTranslator);
}

fn asking(now: Instant, command: &str) -> Conversation {
    let mut conversation = sent("hello", now);
    conversation.ask(&run(command), now, &IdTranslator);
    conversation
}

fn gutter_texts(conversation: &mut Conversation, now: Instant) -> Vec<String> {
    scene_at(conversation, now).panes[1]
        .lines
        .iter()
        .map(text_of)
        .collect()
}

#[test]
fn marks_a_tool_in_the_gutter_then_adds_how_it_ended() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    hear(&mut conversation, ReplyEvent::Text("Reading.".into()), now);

    hear(
        &mut conversation,
        started("t1", ToolKind::Read, "src/lib.rs"),
        now,
    );
    let running = gutter_texts(&mut conversation, after(now, 100));
    hear(
        &mut conversation,
        ended("t1", ToolEnd::Done(Detail::Lines(212))),
        after(now, 200),
    );
    let done = gutter_texts(&mut conversation, after(now, 300));

    assert_eq!(running, vec!["lib.rs"]);
    assert_eq!(done.len(), 1);
    assert!(done[0].starts_with("li"), "{done:?}");
    assert!(done[0].ends_with("  212 gutter-lines"), "{done:?}");
}

#[test]
fn sums_up_a_command_that_ended_in_the_reply() {
    let now = Instant::now();
    let mut conversation = sent("test", now);
    hear(&mut conversation, ReplyEvent::Text("Running.".into()), now);

    hear(
        &mut conversation,
        started("t1", ToolKind::Run, "cargo test"),
        now,
    );
    hear(
        &mut conversation,
        ended("t1", ToolEnd::Done(Detail::Exit(0))),
        after(now, 800),
    );
    hear(
        &mut conversation,
        ReplyEvent::Text("Green.".into()),
        after(now, 900),
    );

    let texts = flow_texts(&scene_at(&mut conversation, after(now, 3_000)));
    let sum = texts
        .iter()
        .position(|row| row == "⌘ cargo test  ✓ · 0decimal-separator8 unit-seconds")
        .unwrap_or_else(|| panic!("{texts:?}"));
    assert_eq!(texts[sum + 1], "Green._");
}

#[test]
fn a_command_run_right_after_another_takes_the_place_of_its_line() {
    let now = Instant::now();
    let mut conversation = sent("check", now);
    hear(&mut conversation, ReplyEvent::Text("Checking.".into()), now);
    hear(
        &mut conversation,
        started("t1", ToolKind::Run, "cargo test"),
        now,
    );
    hear(
        &mut conversation,
        ended("t1", ToolEnd::Done(Detail::Exit(0))),
        after(now, 800),
    );
    hear(
        &mut conversation,
        started("t2", ToolKind::Run, "cargo clippy"),
        after(now, 900),
    );

    hear(
        &mut conversation,
        ended("t2", ToolEnd::Done(Detail::Exit(1))),
        after(now, 2_100),
    );

    let texts = flow_texts(&scene_at(&mut conversation, after(now, 5_000)));
    let commands: Vec<&String> = texts.iter().filter(|row| row.starts_with('⌘')).collect();
    assert_eq!(
        commands,
        ["⌘ cargo clippy  ✗ 1 · 1decimal-separator2 unit-seconds_"],
        "{texts:?}"
    );
}

#[test]
fn a_long_command_is_cut_on_one_row_with_its_result_whole() {
    let now = Instant::now();
    let mut conversation = sent("check", now);
    hear(&mut conversation, ReplyEvent::Text("Checking.".into()), now);
    let command = format!("cargo test {}", "x".repeat(300));
    hear(
        &mut conversation,
        started("t1", ToolKind::Run, &command),
        now,
    );

    hear(
        &mut conversation,
        ended("t1", ToolEnd::Done(Detail::Exit(0))),
        after(now, 800),
    );

    let texts = flow_texts(&scene_at(&mut conversation, after(now, 5_000)));
    let long: Vec<&String> = texts.iter().filter(|row| row.contains("xxx")).collect();
    assert_eq!(long.len(), 1, "{texts:?}");
    assert!(long[0].starts_with("⌘ cargo test xxx"), "{long:?}");
    assert!(
        long[0].ends_with("…  ✓ · 0decimal-separator8 unit-seconds_"),
        "{long:?}"
    );
}

#[test]
fn notes_where_the_conversation_was_summarized() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);

    hear(
        &mut conversation,
        ReplyEvent::Compacted {
            before: 412_000,
            after: 131_000,
        },
        now,
    );

    let texts = flow_texts(&scene_at(&mut conversation, after(now, 3_000)));
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with("conversation-compacted 412k → 131k")),
        "{texts:?}"
    );
}

#[test]
fn asks_in_place_of_the_prompt_and_gives_it_back() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    type_text(&mut conversation, "next", now);

    conversation.ask(&run("rm -r build"), now, &IdTranslator);
    assert!(conversation.is_asking());
    let asked = flow_texts(&scene_at(&mut conversation, after(now, 2_000)));
    conversation.close_question();
    let answered = flow_texts(&scene_at(&mut conversation, after(now, 2_000)));

    assert!(!conversation.is_asking());
    assert!(asked.ends_with(&[
        "› permission-run".to_owned(),
        "rm -r build".to_owned(),
        "permission-allow   permission-deny   grant-command".to_owned(),
    ]));
    assert!(answered.contains(&"› next".to_owned()), "{answered:?}");
}

#[test]
fn keeps_the_draft_as_it_is_while_asking() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);
    type_text(&mut conversation, "draft", now);
    let before = flow_texts(&scene_at(&mut conversation, after(now, 2_000)));
    conversation.ask(&run("ls"), now, &IdTranslator);

    type_text(&mut conversation, "typed", now);
    conversation.compose("^".to_owned());
    conversation.commit("é".to_owned(), now);
    conversation.type_key(
        Intent::Edit(Edit::Insert("x".to_owned())),
        now,
        || None,
        &IdTranslator,
    );
    conversation.close_question();

    let without_caret = |texts: Vec<String>| -> Vec<String> {
        texts.into_iter().filter(|text| text != "_").collect()
    };
    let after_asking = flow_texts(&scene_at(&mut conversation, after(now, 2_000)));
    assert_eq!(without_caret(after_asking), without_caret(before));
}

#[test]
fn a_click_while_asking_changes_nothing() {
    let now = Instant::now();
    let mut conversation = asking(now, "ls");
    let before = flow_texts(&scene_at(&mut conversation, now));
    let layout = crate::conversation::flow::tests::layout();

    conversation.press(crate::conversation::flow::tests::centre(), &layout, now);

    assert_eq!(flow_texts(&scene_at(&mut conversation, now)), before);
}

#[test]
fn answers_only_while_asking() {
    let now = Instant::now();
    let ready = after(now, 300);
    let enter = Key::Named(NamedKey::Enter);
    let mut conversation = asking(now, "ls");

    let answer = conversation.answer_to(&enter, ModifiersState::empty(), ready);
    conversation.close_question();

    assert_eq!(answer, Some(Answer::Allow));
    assert_eq!(
        conversation.answer_to(&enter, ModifiersState::empty(), ready),
        None
    );
}

#[test]
fn asks_nothing_outside_a_reply_and_the_end_of_the_reply_drops_the_question() {
    let now = Instant::now();
    let mut idle = Conversation::default();
    idle.ask(&run("ls"), now, &IdTranslator);
    let mut asked = asking(now, "ls");

    asked.end_reply(None, None, now);

    assert!(!idle.is_asking());
    assert!(!asked.is_asking());
}

#[test]
fn a_reply_that_failed_leaves_its_notice_and_the_prompt_works_again() {
    let now = Instant::now();
    let mut conversation = sent("hello", now);

    conversation.end_reply(Some("reply-overloaded"), None, now);
    type_text(&mut conversation, "again", now);
    let command = press_return(&mut conversation, now);

    assert!(matches!(
        command,
        Some(crate::conversation::Command::Ask(_))
    ));
    let texts = flow_texts(&scene_at(&mut conversation, after(now, 3_000)));
    assert!(texts.contains(&"reply-overloaded".to_owned()), "{texts:?}");
}
