use std::time::Instant;

use dot_api::{Action, Permission};
use winit::keyboard::NamedKey;

use super::*;
use crate::conversation::flow::Command;
use crate::conversation::flow::tests::{press, press_return, type_text};
use crate::conversation::prompter::{Edit, Intent};

fn recall_of(prompts: &[&str]) -> Recall {
    let mut recall = Recall::default();
    for prompt in prompts {
        recall.add(prompt);
    }
    recall
}

fn up(conversation: &mut Conversation, recall: &Recall) -> bool {
    conversation.recall(Motion::Up, recall, Instant::now())
}

fn down(conversation: &mut Conversation, recall: &Recall) -> bool {
    conversation.recall(Motion::Down, recall, Instant::now())
}

#[test]
fn up_writes_the_prompts_sent_before_latest_first() {
    let recall = recall_of(&["first", "second"]);
    let mut conversation = Conversation::default();

    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "second");
    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "first");
    assert!(!up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "first");
}

#[test]
fn down_comes_back_to_what_was_being_written() {
    let recall = recall_of(&["first", "second"]);
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "half wri", Instant::now());
    conversation
        .prompter
        .show("half written", CaretAt::Offset(8));

    up(&mut conversation, &recall);
    up(&mut conversation, &recall);
    assert!(down(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "second");
    assert!(down(&mut conversation, &recall));

    assert_eq!(conversation.prompter.text(), "half written");
    assert_eq!(conversation.prompter.caret_offset(), 8);
    assert!(!down(&mut conversation, &recall));
}

#[test]
fn the_arrows_move_in_a_prompt_of_several_rows_before_they_recall() {
    let recall = recall_of(&["kept"]);
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "one\ntwo", Instant::now());

    assert!(!up(&mut conversation, &recall));
    press(&mut conversation, NamedKey::ArrowUp, Instant::now());
    assert!(!down(&mut conversation, &recall));
    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "kept");
}

#[test]
fn a_recalled_prompt_of_several_rows_lets_up_go_on_and_down_come_back() {
    let recall = recall_of(&["older", "line one\nline two"]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);
    assert_eq!(conversation.prompter.caret_cell().0, 0);
    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "older");
    assert!(down(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "line one\nline two");
    assert_eq!(
        conversation.prompter.caret_offset(),
        "line one\nline two".len()
    );
    assert!(down(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "");
}

#[test]
fn up_on_a_long_recalled_row_stays_on_its_first_row() {
    let long = "word ".repeat(200);
    let recall = recall_of(&["older", long.trim_end()]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);

    assert_eq!(conversation.prompter.caret_cell().0, 0);
    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "older");
}

#[test]
fn typing_keeps_the_recalled_prompt_as_the_one_being_written() {
    let recall = recall_of(&["first", "second"]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);
    type_text(&mut conversation, "!", Instant::now());

    assert_eq!(conversation.prompter.text(), "second!");
    assert!(!down(&mut conversation, &recall));
    assert!(up(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "second");
}

#[test]
fn moving_the_caret_keeps_recalling() {
    let recall = recall_of(&["first", "second"]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);
    conversation.type_key(
        Intent::Edit(Edit::Move(Motion::Left)),
        Instant::now(),
        || None,
        &crate::app::tests::IdTranslator,
    );

    assert!(down(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "");
}

#[test]
fn sending_a_recalled_prompt_ends_the_recall() {
    let recall = recall_of(&["first"]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);
    let sent = press_return(&mut conversation, Instant::now());

    assert!(matches!(sent, Some(Command::Ask(prompt)) if prompt.text == "first"));
    assert_eq!(conversation.recalling, None);
}

#[test]
fn recalls_nothing_without_prompts_or_while_a_question_waits() {
    let mut conversation = Conversation::default();
    assert!(!up(&mut conversation, &Recall::default()));

    let recall = recall_of(&["first"]);
    let mut asked = crate::conversation::flow::tests::sent("ls", Instant::now());
    let permission = Permission {
        tool_id: "t1".to_owned(),
        action: Action::Run {
            command: "ls".to_owned(),
        },
    };
    asked.ask(
        &permission,
        Instant::now(),
        &crate::app::tests::IdTranslator,
    );
    assert!(asked.is_asking());
    assert!(!up(&mut asked, &recall));
}

#[test]
fn down_moves_in_a_recalled_prompt_of_several_rows_before_it_recalls() {
    let recall = recall_of(&["one\ntwo"]);
    let mut conversation = Conversation::default();

    up(&mut conversation, &recall);

    assert_eq!(conversation.prompter.caret_cell().0, 0);
    assert!(!down(&mut conversation, &recall));
    assert_eq!(conversation.prompter.text(), "one\ntwo");
}
