use std::time::Instant;

use dot_api::{Prompt, ReplyEvent, Text};

use super::*;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, press_return, scene_at, sent, type_text,
};
use crate::conversation::prompter::Intent;

fn pause(conversation: &mut Conversation, now: Instant) -> Option<Command> {
    conversation.type_key(Intent::Pause, now, || None, &IdTranslator)
}

fn escape(conversation: &mut Conversation, now: Instant) -> Option<Command> {
    conversation.type_key(Intent::Stop, now, || None, &IdTranslator)
}

fn writing(text: &str, start: Instant) -> Conversation {
    let mut conversation = sent("question", start);
    conversation.hear(
        ReplyEvent::Text(text.to_owned()),
        after(start, 10),
        &IdTranslator,
    );
    conversation
}

fn note_text(command: Option<Command>) -> Option<String> {
    match command {
        Some(Command::Redirect(prompt) | Command::Ask(prompt)) => Some(prompt.text),
        _ => None,
    }
}

#[test]
fn tab_pauses_the_reply_and_sets_the_draft_aside_for_a_note() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "next question", after(start, 20));

    let command = pause(&mut conversation, after(start, 30));

    assert_eq!(command, None);
    assert!(conversation.pause.is_some());
    assert!(conversation.prompter.is_empty());
    let texts = flow_texts(&scene_at(&mut conversation, after(start, 40)));
    assert!(texts.contains(&"‖".to_owned()), "{texts:?}");
    assert!(
        texts.contains(&Text::NotePlaceholder.id().to_owned()),
        "{texts:?}"
    );
    assert!(texts.contains(&"› next question".to_owned()), "{texts:?}");
}

#[test]
fn holds_what_claude_writes_during_the_pause_and_gives_it_back_on_resume() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    pause(&mut conversation, after(start, 20));

    conversation.hear(
        ReplyEvent::Text(" world".into()),
        after(start, 30),
        &IdTranslator,
    );
    let paused = flow_texts(&scene_at(&mut conversation, after(start, 40)));
    let resumed = pause(&mut conversation, after(start, 50));
    scene_at(&mut conversation, after(start, 60));
    let after_resume = flow_texts(&scene_at(&mut conversation, after(start, 2_000)));

    assert!(paused.contains(&"Hello".to_owned()), "{paused:?}");
    assert!(
        !paused.iter().any(|text| text.contains("world")),
        "{paused:?}"
    );
    assert_eq!(resumed, None);
    assert!(
        after_resume.contains(&"Hello world".to_owned()),
        "{after_resume:?}"
    );
    assert!(conversation.pause.is_none());
}

#[test]
fn a_note_stops_the_reply_and_asks_claude_again_below_it() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "my draft", after(start, 15));
    pause(&mut conversation, after(start, 20));
    type_text(&mut conversation, "shorter please", after(start, 30));

    let command = press_return(&mut conversation, after(start, 40));

    assert!(matches!(command, Some(Command::Redirect(_))));
    assert_eq!(note_text(command), Some("shorter please".to_owned()));
    let texts = flow_texts(&scene_at(&mut conversation, after(start, 2_000)));
    assert!(texts.contains(&"‖ shorter please".to_owned()), "{texts:?}");
    assert!(texts.contains(&"› my draft".to_owned()), "{texts:?}");
    assert!(conversation.transcript.is_live());
    assert_eq!(conversation.transcript.exchanges(), 2);
}

#[test]
fn a_reply_that_ended_during_the_pause_ends_on_resume() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    pause(&mut conversation, after(start, 20));

    let sent_meanwhile = conversation.end_reply(None, Some(30), after(start, 30));
    let still_live = conversation.transcript.is_live();
    let resumed = pause(&mut conversation, after(start, 40));

    assert_eq!(sent_meanwhile, None);
    assert!(still_live);
    assert_eq!(resumed, None);
    assert!(!conversation.transcript.is_live());
}

#[test]
fn a_note_after_the_reply_ended_is_asked_as_a_prompt() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    pause(&mut conversation, after(start, 20));
    conversation.end_reply(None, Some(30), after(start, 30));
    type_text(&mut conversation, "and then?", after(start, 35));

    let command = press_return(&mut conversation, after(start, 40));

    assert!(matches!(command, Some(Command::Ask(_))));
    assert_eq!(note_text(command), Some("and then?".to_owned()));
    assert!(conversation.transcript.is_live());
}

#[test]
fn escape_during_the_pause_drops_the_note_gives_the_draft_back_and_stops() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "my draft", after(start, 15));
    pause(&mut conversation, after(start, 20));
    type_text(&mut conversation, "a note", after(start, 25));
    conversation.hear(
        ReplyEvent::Text(" world".into()),
        after(start, 30),
        &IdTranslator,
    );

    let command = escape(&mut conversation, after(start, 40));

    assert_eq!(command, Some(Command::Stop));
    assert!(conversation.pause.is_none());
    scene_at(&mut conversation, after(start, 50));
    let texts = flow_texts(&scene_at(&mut conversation, after(start, 2_000)));
    assert!(texts.contains(&"Hello world".to_owned()), "{texts:?}");
    assert!(texts.contains(&"› my draft".to_owned()), "{texts:?}");
    assert!(
        !texts.iter().any(|text| text.contains("a note")),
        "{texts:?}"
    );
}

#[test]
fn tab_does_nothing_outside_a_reply_or_on_a_note_being_written() {
    let start = Instant::now();
    let mut idle = Conversation::default();
    let mut paused = writing("Hello", start);
    pause(&mut paused, after(start, 10));
    type_text(&mut paused, "a note", after(start, 20));

    assert_eq!(pause(&mut idle, start), None);
    assert!(idle.pause.is_none());
    assert_eq!(pause(&mut paused, after(start, 30)), None);
    assert!(paused.pause.is_some());
}

#[test]
fn return_during_a_reply_queues_the_draft_and_sends_it_once_the_reply_ends() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "first", after(start, 20));
    let queued = press_return(&mut conversation, after(start, 30));
    type_text(&mut conversation, "second", after(start, 40));
    press_return(&mut conversation, after(start, 50));
    let empty = press_return(&mut conversation, after(start, 55));

    let texts = flow_texts(&scene_at(&mut conversation, after(start, 60)));
    let sent_after = conversation.end_reply(None, Some(12), after(start, 70));

    assert_eq!(queued, None);
    assert_eq!(empty, None);
    assert!(conversation.prompter.is_empty());
    assert!(texts.contains(&"› first".to_owned()), "{texts:?}");
    let last_queued = format!("second · {}", Text::QueuedAfterTheReply.id());
    assert!(
        texts.contains(&format!("  {last_queued}"))
            || texts.iter().any(|text| text.ends_with(&last_queued)),
        "{texts:?}"
    );
    assert_eq!(
        sent_after.map(|prompt| prompt.text),
        Some("first\nsecond".to_owned())
    );
    assert!(conversation.transcript.is_live());
    assert!(conversation.queued.is_none());
}

#[test]
fn says_the_reply_is_paused_and_a_prompt_waits() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "later", after(start, 10));
    press_return(&mut conversation, after(start, 20));
    let layout = crate::conversation::flow::tests::layout();

    let writing_with_a_queue = conversation.activity(&layout, after(start, 30), &IdTranslator);
    pause(&mut conversation, after(start, 40));
    let paused = conversation.activity(&layout, after(start, 50), &IdTranslator);

    assert_eq!(
        writing_with_a_queue.text.as_deref(),
        Some("state-writing · state-queued")
    );
    assert_eq!(paused.text.as_deref(), Some("state-paused · state-queued"));
}

#[test]
fn refuses_to_queue_images_past_what_one_prompt_carries_and_keeps_the_draft() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    for index in 0..Prompt::MOST_IMAGES {
        let image = crate::conversation::prompter::image_attachment(&format!("{index}.png"));
        conversation.prompter.attach(image).unwrap();
    }
    press_return(&mut conversation, after(start, 30));
    type_text(&mut conversation, "one more", after(start, 40));
    let extra = crate::conversation::prompter::image_attachment("extra.png");
    conversation.prompter.attach(extra).unwrap();

    press_return(&mut conversation, after(start, 50));

    let queued = conversation.queued.as_ref().unwrap();
    assert_eq!(queued.attachments.len(), Prompt::MOST_IMAGES);
    assert!(!conversation.prompter.is_empty());
    let texts = flow_texts(&scene_at(&mut conversation, after(start, 60)));
    assert!(
        texts
            .iter()
            .any(|text| text.contains(Text::ImageTooMany.id())),
        "{texts:?}"
    );
}

#[test]
fn counts_the_rows_of_the_queue_once_for_a_width_and_again_once_it_changes() {
    let queued = Queued {
        text: "one two three four five".to_owned(),
        ..Queued::default()
    };

    let wide = queued.rows_at(dot_transcript::TextWidth::new(40));
    let narrow = queued.rows_at(dot_transcript::TextWidth::new(8));

    let expected = dot_transcript::wrap(&queued.text, 8).len();
    assert_eq!(wide, 1);
    assert!(expected > 1);
    assert_eq!(narrow, expected);
    assert_eq!(queued.rows_at(dot_transcript::TextWidth::new(8)), expected);
}

#[test]
fn counts_the_rows_of_the_queue_again_once_a_prompt_joins_it() {
    let start = Instant::now();
    let mut conversation = writing("Hello", start);
    type_text(&mut conversation, "first", after(start, 20));
    press_return(&mut conversation, after(start, 30));
    let width = conversation.prompter.width();
    let one = conversation.queued.as_ref().unwrap().rows_at(width);

    type_text(&mut conversation, "second", after(start, 40));
    press_return(&mut conversation, after(start, 50));

    assert_eq!(one, 1);
    assert_eq!(conversation.queued.as_ref().unwrap().rows_at(width), 2);
}
