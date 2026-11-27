use std::borrow::Cow;
use std::time::Duration;

use dot_api::{Asked, Detail, Ending, Failure, Reply, ReplyEvent, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, press_return, scene_at, type_text,
};

const SETTLED: u64 = 10_000;

fn at(milliseconds: u64, happened: Happened<'static>) -> Record<'static> {
    Record {
        at: Duration::from_millis(milliseconds),
        happened,
    }
}

fn asked(milliseconds: u64, text: &str, opening: Opening) -> Record<'static> {
    at(
        milliseconds,
        Happened::Asked(Asked {
            text: Cow::Owned(text.to_owned()),
            opening,
            images: Vec::new(),
        }),
    )
}

fn heard(milliseconds: u64, event: ReplyEvent) -> Record<'static> {
    at(milliseconds, Happened::Heard(Cow::Owned(event)))
}

fn ended(milliseconds: u64, failure: Option<Failure>) -> Record<'static> {
    at(
        milliseconds,
        Happened::Ended(Ending {
            spent: Reply {
                output_tokens: 12,
                ..Reply::default()
            },
            failure,
        }),
    )
}

fn failed(text: Text, cause: Option<&str>) -> Failure {
    Failure {
        text,
        cause: cause.map(str::to_owned),
    }
}

fn texts_of(records: Vec<Record<'static>>) -> Vec<String> {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    conversation.replay(records, start, &IdTranslator);
    flow_texts(&scene_at(&mut conversation, after(start, SETTLED)))
}

fn shows(texts: &[String], text: &str) -> bool {
    texts.iter().any(|shown| shown.contains(text))
}

#[test]
fn a_replayed_exchange_reads_as_it_did_live() {
    let start = Instant::now();
    let run = ToolUse {
        kind: ToolKind::Run,
        subject: "cargo test".to_owned(),
    };
    let events = [
        (100, ReplyEvent::Text("Je lance les tests.".to_owned())),
        (
            200,
            ReplyEvent::ToolStarted {
                id: "t1".to_owned(),
                tool: run,
            },
        ),
        (
            1_000,
            ReplyEvent::ToolEnded {
                id: "t1".to_owned(),
                end: ToolEnd::Done(Detail::Exit(0)),
            },
        ),
        (1_100, ReplyEvent::Text("Tout passe.".to_owned())),
    ];

    let mut live = Conversation::default();
    type_text(&mut live, "Lance les tests", start);
    press_return(&mut live, start);
    for (milliseconds, event) in events.clone() {
        live.hear(event, after(start, milliseconds), &IdTranslator);
    }
    live.end_reply(None, Some(12), after(start, 1_200));

    let mut records = vec![asked(0, "Lance les tests", Opening::Prompt)];
    records.extend(
        events
            .into_iter()
            .map(|(milliseconds, event)| heard(milliseconds, event)),
    );
    records.push(ended(1_200, None));
    let mut replayed = Conversation::default();
    replayed.replay(records, start, &IdTranslator);

    let later = after(start, SETTLED);
    let live_texts = flow_texts(&scene_at(&mut live, later));
    assert!(shows(&live_texts, "0decimal-separator8"), "{live_texts:?}");
    assert_eq!(flow_texts(&scene_at(&mut replayed, later)), live_texts);
    assert!(!replayed.is_live());
}

#[test]
fn a_reply_the_user_stopped_ends_with_its_notice() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        heard(10, ReplyEvent::Text("Bonj".to_owned())),
        ended(20, Some(failed(ReplyFailure::Stopped.text(), None))),
    ]);

    assert!(shows(&texts, "reply-stopped"), "{texts:?}");
}

#[test]
fn a_note_sent_in_place_of_a_reply_ends_it_without_a_word() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        heard(10, ReplyEvent::Text("Bonj".to_owned())),
        ended(20, Some(failed(ReplyFailure::Stopped.text(), None))),
        asked(30, "plutôt en anglais", Opening::Redirect),
        heard(40, ReplyEvent::Text("Hello".to_owned())),
        ended(50, None),
    ]);

    assert!(!shows(&texts, "reply-stopped"), "{texts:?}");
    assert!(shows(&texts, "plutôt en anglais"), "{texts:?}");
    assert!(shows(&texts, "Hello"), "{texts:?}");
}

#[test]
fn a_stopped_reply_followed_by_a_prompt_keeps_its_notice() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        ended(20, Some(failed(ReplyFailure::Stopped.text(), None))),
        asked(30, "Encore", Opening::Prompt),
        ended(50, None),
    ]);

    assert!(shows(&texts, "reply-stopped"), "{texts:?}");
    assert!(shows(&texts, "Encore"), "{texts:?}");
}

#[test]
fn a_failed_reply_ends_with_its_notice_and_its_cause() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        ended(20, Some(failed(Text::ReplyOverloaded, Some("HTTP 529")))),
    ]);

    assert!(shows(&texts, "reply-overloaded"), "{texts:?}");
    assert!(shows(&texts, "HTTP 529"), "{texts:?}");
}

#[test]
fn a_reply_cut_short_by_a_crash_ends_stopped() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    conversation.replay(
        vec![
            asked(0, "Salut", Opening::Prompt),
            heard(10, ReplyEvent::Text("Bonj".to_owned())),
        ],
        start,
        &IdTranslator,
    );

    assert!(!conversation.is_live());
    let texts = flow_texts(&scene_at(&mut conversation, after(start, SETTLED)));
    assert!(shows(&texts, "reply-stopped"), "{texts:?}");
}

#[test]
fn a_prompt_after_a_reply_cut_short_ends_it_stopped() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        heard(10, ReplyEvent::Text("Bonj".to_owned())),
        asked(30, "Encore", Opening::Prompt),
        ended(50, None),
    ]);

    assert!(shows(&texts, "reply-stopped"), "{texts:?}");
    assert!(shows(&texts, "Encore"), "{texts:?}");
}

#[test]
fn a_fresh_start_shows_its_notice() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        ended(20, None),
        at(30, Happened::StartedOver),
    ]);

    assert!(shows(&texts, "conversation-restarted"), "{texts:?}");
}

#[test]
fn names_the_images_sent_without_keeping_them() {
    let texts = texts_of(vec![
        at(
            0,
            Happened::Asked(Asked {
                text: Cow::Owned("Regarde".to_owned()),
                opening: Opening::Prompt,
                images: vec![ImageLabel {
                    name: "shot.png".to_owned(),
                    bytes: 2_048,
                }],
            }),
        ),
        ended(20, None),
    ]);

    assert!(
        shows(&texts, "image-label  shot.png  2 size-kilobytes"),
        "{texts:?}"
    );
}

#[test]
fn ignores_what_arrives_outside_a_reply() {
    let texts = texts_of(vec![
        heard(0, ReplyEvent::Text("orphan".to_owned())),
        ended(5, None),
        asked(10, "Salut", Opening::Prompt),
        ended(20, None),
        heard(30, ReplyEvent::Text("late".to_owned())),
        at(40, Happened::Carried(dot_api::Carried::default())),
    ]);

    assert!(!shows(&texts, "orphan"), "{texts:?}");
    assert!(!shows(&texts, "late"), "{texts:?}");
}

#[test]
fn a_replayed_conversation_has_exchanges_and_is_no_longer_untouched() {
    let mut conversation = Conversation::default();
    assert!(conversation.is_untouched());
    assert!(!conversation.has_exchanges());

    conversation.replay(
        vec![asked(0, "Salut", Opening::Prompt), ended(20, None)],
        Instant::now(),
        &IdTranslator,
    );

    assert!(conversation.has_exchanges());
    assert!(!conversation.is_untouched());
}

#[test]
fn a_conversation_being_written_is_not_untouched() {
    let mut conversation = Conversation::default();

    type_text(&mut conversation, "a", Instant::now());

    assert!(!conversation.is_untouched());
}

#[test]
fn remembers_how_the_last_prompt_was_sent() {
    let mut conversation = Conversation::default();
    assert_eq!(conversation.opened(), crate::reply::Opened::default());

    type_text(&mut conversation, "Salut", Instant::now());
    press_return(&mut conversation, Instant::now());

    assert_eq!(conversation.opened().opening, Opening::Prompt);
    assert_eq!(conversation.opened().images, Vec::<ImageLabel>::new());
}

#[test]
fn remembers_the_images_and_the_note_of_the_last_opening() {
    let mut conversation = Conversation::default();
    conversation
        .attach(
            "shot.png",
            dot_api::Image::from_bytes(b"\x89PNG\r\n\x1a\n".to_vec()).unwrap(),
            &IdTranslator,
        )
        .unwrap();
    type_text(&mut conversation, "Regarde", Instant::now());
    press_return(&mut conversation, Instant::now());

    let opened = conversation.opened();

    assert_eq!(opened.opening, Opening::Prompt);
    assert_eq!(
        opened.images,
        [ImageLabel {
            name: "shot.png".to_owned(),
            bytes: 8,
        }]
    );
}

#[test]
fn shows_no_end_nor_tool_outside_a_reply() {
    let texts = texts_of(vec![
        heard(
            0,
            ReplyEvent::ToolStarted {
                id: "t1".to_owned(),
                tool: ToolUse {
                    kind: ToolKind::Run,
                    subject: "orphan-command".to_owned(),
                },
            },
        ),
        heard(
            1,
            ReplyEvent::ToolEnded {
                id: "t1".to_owned(),
                end: ToolEnd::Done(Detail::Exit(0)),
            },
        ),
        ended(2, Some(failed(Text::ReplyOverloaded, Some("orphan-cause")))),
        asked(10, "Salut", Opening::Prompt),
        ended(20, None),
        ended(30, Some(failed(Text::ReplyOverloaded, Some("late-cause")))),
    ]);

    assert!(!shows(&texts, "orphan-command"), "{texts:?}");
    assert!(!shows(&texts, "orphan-cause"), "{texts:?}");
    assert!(!shows(&texts, "late-cause"), "{texts:?}");
}

#[test]
fn nothing_joins_a_reply_once_it_was_stopped() {
    let texts = texts_of(vec![
        asked(0, "Salut", Opening::Prompt),
        ended(20, Some(failed(ReplyFailure::Stopped.text(), None))),
        heard(30, ReplyEvent::Text("late".to_owned())),
    ]);

    assert!(shows(&texts, "reply-stopped"), "{texts:?}");
    assert!(!shows(&texts, "late"), "{texts:?}");
}
