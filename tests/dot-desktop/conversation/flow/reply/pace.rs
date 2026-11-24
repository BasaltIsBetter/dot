use dot_api::{Detail, FileChange, Hunk, PatchLine, Prompt, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::conversation::flow::tests::{IdTranslator, after, press_return, sent, type_text};

fn paced(question: &str, start: Instant) -> Conversation {
    let mut conversation = sent(question, start);
    conversation.pace_replies();
    conversation
}

fn shown(conversation: &Conversation) -> String {
    let transcript = &conversation.transcript;
    (0..transcript.total())
        .filter_map(|row| transcript.row_text(row))
        .map(|row| row.text)
        .collect::<Vec<_>>()
        .join("|")
}

fn hear(conversation: &mut Conversation, event: ReplyEvent, now: Instant) {
    conversation.hear(event, now, &IdTranslator);
}

fn flow(conversation: &mut Conversation, now: Instant) -> Option<Ended> {
    conversation.flow(now, &IdTranslator)
}

fn flow_until_ended(conversation: &mut Conversation, start: Instant) -> Option<Ended> {
    (1..=20).find_map(|frame| flow(conversation, after(start, 100 * frame)))
}

fn text(piece: &str) -> ReplyEvent {
    ReplyEvent::Text(piece.to_owned())
}

fn started(id: &str) -> ReplyEvent {
    ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool: ToolUse {
            kind: ToolKind::Edit,
            subject: "src/a.rs".to_owned(),
        },
    }
}

fn changed(id: &str, lines: usize) -> ReplyEvent {
    written(id, lines, false)
}

fn written(id: &str, lines: usize, created: bool) -> ReplyEvent {
    ReplyEvent::ToolChanged {
        id: id.to_owned(),
        change: Box::new(FileChange {
            path: "src/a.rs".to_owned(),
            created,
            hunks: vec![Hunk {
                old_start: 1,
                new_start: 1,
                lines: (0..lines)
                    .map(|line| PatchLine::Added(format!("l{line}")))
                    .collect(),
            }],
            siblings: None,
        }),
    }
}

#[test]
fn shows_a_reply_word_by_word_however_it_arrives() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);

    hear(&mut conversation, text("un deux trois quatre"), start);
    let first = shown(&conversation);
    flow(&mut conversation, after(start, 100));
    let second = shown(&conversation);
    flow(&mut conversation, after(start, 200));
    flow(&mut conversation, after(start, 300));

    assert!(!first.contains("un"), "{first}");
    assert!(second.ends_with("un deux"), "{second}");
    assert!(shown(&conversation).ends_with("un deux trois quatre"));
    assert!(!conversation.is_pacing());
}

#[test]
fn keeps_pacing_while_something_waits() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);

    hear(&mut conversation, text("un deux trois quatre"), start);

    assert!(conversation.is_pacing());
}

#[test]
fn shows_what_follows_the_text_only_once_the_text_is_shown() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("Je regarde le fichier."), start);

    hear(&mut conversation, started("t1"), after(start, 1));
    let early = conversation.transcript.running_tool("t1").is_some();
    flow(&mut conversation, after(start, 100));
    let midway = conversation.transcript.running_tool("t1").is_some();
    flow(&mut conversation, after(start, 200));
    flow(&mut conversation, after(start, 300));

    assert!(!early && !midway);
    assert!(conversation.transcript.running_tool("t1").is_some());
}

#[test]
fn waits_for_a_block_of_code_to_be_written_before_the_text_after_it() {
    let start = Instant::now();
    let mut conversation = paced("fix", start);
    hear(&mut conversation, started("t1"), start);
    hear(&mut conversation, changed("t1", 4), start);
    hear(&mut conversation, text("Fait."), start);

    flow(&mut conversation, after(start, 100));
    flow(&mut conversation, after(start, 200));
    let writing = shown(&conversation);
    flow(&mut conversation, after(start, 300));
    flow(&mut conversation, after(start, 400));

    assert!(!writing.contains("Fait."), "{writing}");
    assert!(shown(&conversation).ends_with("Fait."));
}

#[test]
fn stops_waiting_for_blocks_once_it_lags_a_second_and_a_half_behind() {
    let start = Instant::now();
    let mut conversation = paced("fix", start);
    hear(&mut conversation, started("t1"), start);
    hear(&mut conversation, written("t1", 30, true), start);
    hear(&mut conversation, text("Fait."), start);

    flow(&mut conversation, after(start, 1_400));
    let waiting = shown(&conversation);
    flow(&mut conversation, after(start, 1_500));
    flow(&mut conversation, after(start, 1_600));

    assert!(!waiting.contains("Fait."));
    assert!(shown(&conversation).ends_with("Fait."));
}

#[test]
fn ends_the_reply_once_everything_before_its_end_is_shown() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("un deux trois"), start);

    let at_once = conversation.end_reply(None, Some(3), start);
    let still_live = conversation.is_live();
    let early = flow(&mut conversation, after(start, 50));
    let ended = flow_until_ended(&mut conversation, start);

    assert!(at_once.is_none() && early.is_none());
    assert!(still_live);
    assert!(shown(&conversation).ends_with("un deux trois"));
    assert!(ended.is_some_and(|ended| ended.next.is_none()));
    assert!(!conversation.is_live());
}

#[test]
fn sends_the_prompt_queued_behind_a_reply_once_it_is_shown() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("un deux trois"), start);
    type_text(&mut conversation, "next", start);
    press_return(&mut conversation, start);
    conversation.end_reply(None, Some(3), start);

    let ended = flow_until_ended(&mut conversation, start);

    let next: Option<Prompt> = ended.and_then(|ended| ended.next);
    assert_eq!(next.map(|prompt| prompt.text), Some("next".to_owned()));
}

#[test]
fn ends_at_once_when_nothing_waits() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("un"), start);
    flow(&mut conversation, after(start, 100));

    conversation.end_reply(None, Some(1), after(start, 100));

    assert!(!conversation.is_live());
    assert!(flow(&mut conversation, after(start, 200)).is_none());
}

#[test]
fn shows_everything_at_once_when_flushed() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("un deux trois quatre cinq"), start);
    hear(&mut conversation, started("t1"), start);
    conversation.end_reply(None, Some(5), start);

    let ended = conversation.flush(start, &IdTranslator);

    assert!(ended.is_some());
    assert!(shown(&conversation).contains("un deux trois quatre cinq"));
    assert!(!conversation.is_live());
    assert!(!conversation.is_pacing());
}

#[test]
fn shows_nothing_more_while_paused_and_everything_waiting_on_resume() {
    let start = Instant::now();
    let mut conversation = paced("hi", start);
    hear(&mut conversation, text("un deux trois quatre"), start);
    conversation.pause();

    hear(&mut conversation, text(" cinq"), after(start, 50));
    flow(&mut conversation, after(start, 500));
    let paused = shown(&conversation);
    conversation.resume(after(start, 600), &IdTranslator);

    assert!(!paused.contains("un"), "{paused}");
    assert!(shown(&conversation).contains("un deux trois quatre cinq"));
    assert!(!conversation.is_pacing());
}

#[test]
fn shows_at_once_without_a_pace() {
    let start = Instant::now();
    let mut conversation = sent("hi", start);

    hear(&mut conversation, text("un deux trois"), start);

    assert!(shown(&conversation).ends_with("un deux trois"));
    assert!(!conversation.is_pacing());
    assert!(flow(&mut conversation, after(start, 100)).is_none());
}

#[test]
fn lets_what_follows_a_block_through_the_moment_its_lines_have_their_rows() {
    let start = Instant::now();
    let mut conversation = paced("fix", start);
    hear(&mut conversation, started("t1"), start);
    hear(&mut conversation, changed("t1", 4), start);
    let end = ToolEnd::Done(Detail::Changes {
        added: 4,
        removed: 0,
    });
    hear(
        &mut conversation,
        ReplyEvent::ToolEnded {
            id: "t1".to_owned(),
            end,
        },
        start,
    );

    flow(&mut conversation, after(start, 279));
    let held = conversation.transcript.running_tool("t1").is_some();
    flow(&mut conversation, after(start, 280));

    assert!(held);
    assert!(conversation.transcript.running_tool("t1").is_none());
}
