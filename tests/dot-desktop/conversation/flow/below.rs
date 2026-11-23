use std::time::Instant;

use dot_api::{Action, Permission, ReplyEvent};

use super::*;
use crate::conversation::flow::tests::{IdTranslator, after, sent, stage, text_of, type_text};

#[test]
fn rests_on_the_row_being_typed_below_the_conversation() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "a\nb\nc", start);

    let plan = conversation.plan(&stage());

    assert_eq!(conversation.below().draft, 0);
    assert_eq!(plan.resting, 2);
    assert_eq!(plan.last, 2);
    assert_eq!(plan.writing, None);
}

#[test]
fn types_one_blank_row_below_the_conversation() {
    let start = Instant::now();
    let mut conversation = sent("hello", start);
    conversation.hear(ReplyEvent::Text("Hi".to_owned()), start, &IdTranslator);
    conversation.end_reply(None, None, start);

    assert_eq!(conversation.below().draft, 4);
    assert_eq!(conversation.caret_row(), 4);
}

#[test]
fn rests_on_the_reply_while_the_prompt_below_fits_above_the_bottom() {
    let start = Instant::now();
    let conversation = sent("hello", start);

    let plan = conversation.plan(&stage());

    assert_eq!(plan.resting, 0);
    assert_eq!(plan.writing, Some(0));
    assert_eq!(conversation.below().draft, 2);
    assert_eq!(plan.last, 2);
}

#[test]
fn rests_lower_once_the_prompt_below_passes_the_bottom() {
    let start = Instant::now();
    let mut conversation = sent("hello", start);
    let reply = "row\n".repeat(30);
    conversation.hear(ReplyEvent::Text(reply), after(start, 10), &IdTranslator);

    let stage = stage();
    let plan = conversation.plan(&stage);

    assert_eq!(plan.resting, plan.last - stage.rows_below());
    assert_eq!(plan.writing, Some(conversation.transcript.total() - 1));
}

#[test]
fn shows_only_the_marker_below_a_reply_while_nothing_is_typed() {
    let start = Instant::now();
    let conversation = sent("hello", start);
    let stage = stage();

    let lines = conversation.below_lines(&stage, 0.0, start, &IdTranslator);

    assert_eq!(lines.iter().map(text_of).collect::<Vec<_>>(), vec!["›"]);
}

#[test]
fn shows_what_is_typed_below_a_reply_with_its_caret() {
    let start = Instant::now();
    let mut conversation = sent("hello", start);
    type_text(&mut conversation, "next", start);
    let stage = stage();

    let lines = conversation.below_lines(&stage, 0.0, start, &IdTranslator);

    assert_eq!(text_of(&lines[0]), "› next");
    assert!(lines.iter().any(|placed| text_of(placed) == "_"));
}

#[test]
fn shows_the_question_of_a_permission_in_place_of_the_prompt() {
    let start = Instant::now();
    let mut conversation = sent("hello", start);
    let permission = Permission {
        tool_id: "t1".to_owned(),
        action: Action::Run {
            command: "ls".to_owned(),
        },
    };
    conversation.ask(&permission, start, &IdTranslator);
    let stage = stage();

    let lines = conversation.below_lines(&stage, 0.0, start, &IdTranslator);
    let plan = conversation.plan(&stage);

    assert_eq!(text_of(&lines[0]), "› permission-run");
    assert_eq!(lines.len(), 3);
    assert_eq!(plan.last, conversation.below().draft + 2);
}
