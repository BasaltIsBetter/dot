use dot_api::{ReplyEvent, ToolKind, ToolUse};
use dot_transcript::{Ink, Span};

use super::*;
use crate::canvas::TextSize;
use crate::conversation::flow::tests::{
    IdTranslator, NARROW, RETINA, after, answered, scene_at, sent, stage, text_of,
};

#[test]
fn the_band_follows_the_row_claude_writes_on_then_comes_back() {
    let start = Instant::now();
    let mut conversation = sent("list", start);
    let resting = scene_at(&mut conversation, start).grounds[0].top;

    conversation.hear(
        ReplyEvent::Text("a\nb\nc\nd".to_owned()),
        after(start, 100),
        &IdTranslator,
    );
    scene_at(&mut conversation, after(start, 150));
    let writing = scene_at(&mut conversation, after(start, 600)).grounds[0].top;
    conversation.end_reply(None, None, after(start, 700));
    scene_at(&mut conversation, after(start, 700));
    let back = scene_at(&mut conversation, after(start, 1_500)).grounds[0].top;

    let row = stage().row;
    assert!(
        ((writing - resting) - 3.0 * row).abs() < 2.0,
        "{resting} {writing}"
    );
    assert!((back - resting).abs() < 1.0);
}

#[test]
fn shows_the_tick_until_the_first_prompt_is_sent() {
    let start = Instant::now();
    let mut fresh = Conversation::default();
    let mut replied = answered(1, start);

    let before = scene_at(&mut fresh, start);
    let after_a_reply = scene_at(&mut replied, after(start, 2_000));

    let tick = |scene: &Scene| {
        let stage = stage();
        scene.marks.iter().any(|mark| {
            (mark.left - stage.points(16.0)).abs() < 0.5
                && (mark.width - stage.points(16.0)).abs() < 0.5
        })
    };
    assert_eq!(before.marks.len(), 1);
    assert!(tick(&before));
    assert!(!tick(&after_a_reply));
}

#[test]
fn draws_the_band_and_fades_the_bottom_into_the_status_bar() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    let scene = scene_at(&mut conversation, start);

    assert_eq!(scene.grounds.len(), 1);
    assert_ne!(scene.veils, Vec::new());
    assert!(scene.veils.iter().all(|veil| veil.top < stage().floor));
}

#[test]
fn shows_no_tool_beside_the_reply() {
    let start = Instant::now();
    let mut conversation = sent("look", start);
    conversation.hear(
        ReplyEvent::Text("Reading.".to_owned()),
        start,
        &IdTranslator,
    );
    let read = ToolUse {
        kind: ToolKind::Read,
        subject: "crates/a/layout.rs".to_owned(),
    };
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool: read,
        },
        after(start, 10),
        &IdTranslator,
    );

    let scene = scene_at(&mut conversation, after(start, 2_000));

    let [left, _] = stage().gutter.unwrap();
    assert!(scene.panes.iter().all(|pane| pane.left < left));
    assert!(
        scene
            .panes
            .iter()
            .flat_map(|pane| &pane.lines)
            .all(|placed| !text_of(placed).contains("layout.rs"))
    );
    assert!(scene.marks.iter().all(|mark| mark.left < left));
}

#[test]
fn a_narrow_window_has_no_gutter() {
    let start = Instant::now();
    let mut conversation = sent("look", start);
    let narrow = Layout::reflowed(NARROW, RETINA);
    conversation.prepare(&narrow, start, true);

    let scene = conversation.scene(&narrow, start, &IdTranslator);

    assert_eq!(scene.panes.len(), 1);
}

fn help_shown(conversation: &mut Conversation, now: Instant) -> bool {
    let scene = scene_at(conversation, now);
    scene.panes.iter().any(|pane| {
        pane.lines
            .iter()
            .any(|placed| text_of(placed).starts_with("help-send-key"))
    })
}

#[test]
fn shows_no_help_while_claude_replies_however_calm() {
    let start = Instant::now();
    let mut conversation = sent("look", start);

    assert!(!help_shown(&mut conversation, after(start, 20_000)));
}

#[test]
fn shows_no_help_to_who_reads_back() {
    let start = Instant::now();
    let mut conversation = answered(3, start);
    conversation.reader.scroll_by(-4.0);

    assert!(!help_shown(&mut conversation, after(start, 20_000)));
}

#[test]
fn places_the_help_below_the_line_from_the_left_edge() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    let scene = scene_at(&mut conversation, after(start, 6_000));

    let help = scene
        .panes
        .iter()
        .find(|pane| {
            pane.lines
                .iter()
                .any(|placed| text_of(placed).starts_with("help-send-key"))
        })
        .unwrap();
    let stage = stage();
    let tops: Vec<f32> = help.lines.iter().map(|placed| placed.top).collect();
    assert!((help.left - stage.points(24.0)).abs() < 0.5);
    let row = stage.points(22.0);
    let first = stage.centre + stage.points(56.0);
    let layout = crate::conversation::flow::tests::layout();
    assert_eq!(tops[0], layout.text_top(TextSize::Small, first, row));
    assert_eq!(tops[1], layout.text_top(TextSize::Small, first + row, row));
}

#[test]
fn is_animating_while_anything_moves_and_still_once_it_rests() {
    let start = Instant::now();
    let mut typing = Conversation::default();
    crate::conversation::flow::tests::type_text(&mut typing, "a", start);
    let mut arriving = sent("look", start);
    arriving.hear(
        ReplyEvent::Text("Hello".into()),
        after(start, 100),
        &IdTranslator,
    );
    let mut resting = answered(1, start);

    assert!(scene_at(&mut typing, after(start, 100)).animating);
    assert!(scene_at(&mut arriving, after(start, 200)).animating);
    assert!(!scene_at(&mut resting, after(start, 60_000)).animating);
}

#[test]
fn rolls_the_suggestions_only_when_idle_focused_and_empty() {
    let start = Instant::now();
    let mut idle = Conversation::default();
    let mut typed = Conversation::default();
    crate::conversation::flow::tests::type_text(&mut typed, "a", start);
    let mut replying = sent("look", start);
    let layout = crate::conversation::flow::tests::layout();

    for now in [start, after(start, 4_100)] {
        idle.prepare(&layout, now, true);
        typed.prepare(&layout, now, true);
        replying.prepare(&layout, now, true);
    }
    let mut unfocused = Conversation::default();
    for now in [start, after(start, 4_100)] {
        unfocused.prepare(&layout, now, false);
    }

    assert!(idle.suggestions.is_rolling(after(start, 4_100)));
    assert!(!typed.suggestions.is_rolling(after(start, 4_100)));
    assert!(!replying.suggestions.is_rolling(after(start, 4_100)));
    assert!(!unfocused.suggestions.is_rolling(after(start, 4_100)));
    assert!(scene_at(&mut idle, after(start, 4_100)).animating);
}

#[test]
fn wakes_for_the_instrument_while_claude_replies() {
    let start = Instant::now();
    let conversation = sent("look", start);

    let woken = conversation.next_change(after(start, 50));

    assert_eq!(woken, Some(after(start, 100)));
}

#[test]
fn puts_the_help_at_the_bottom_of_a_narrow_window() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    let narrow = Layout::reflowed(NARROW, RETINA);
    let later = after(start, 6_000);
    conversation.prepare(&narrow, start, true);
    conversation.prepare(&narrow, later, true);

    let scene = conversation.scene(&narrow, later, &IdTranslator);

    let help = scene
        .panes
        .iter()
        .find(|pane| {
            pane.lines
                .iter()
                .any(|placed| text_of(placed).starts_with("help-send-key"))
        })
        .unwrap();
    let stage = Stage::of(&narrow);
    let row = stage.points(22.0);
    let count = help.lines.len();
    let first = stage.floor - stage.points(64.0) - row * f32::from(u16::try_from(count).unwrap());
    assert_eq!(count, 7);
    assert_eq!(
        help.lines[0].top,
        narrow.text_top(TextSize::Small, first, row)
    );
}

const VIOLET: [u8; 3] = [0xa9, 0x70, 0xff];

fn colourful() -> Colours {
    Colours {
        tool: Some(VIOLET),
        states: true,
    }
}

struct CommandKeys;

impl Translator for CommandKeys {
    fn translate(&self, text: dot_api::Text) -> std::borrow::Cow<'_, str> {
        match text {
            Text::HelpSwitchKey => "⌘↑ ⌘↓".into(),
            Text::HelpSettingsKey => "⌘ molette".into(),
            other => other.id().into(),
        }
    }
    fn count(&self, text: Text, _count: u64) -> std::borrow::Cow<'_, str> {
        self.translate(text)
    }
    fn use_language(&self, _language: dot_api::Language) {}
}

#[test]
fn leaves_the_keys_of_the_help_grey_though_they_start_like_a_command() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    conversation.colour(colourful());
    let layout = crate::conversation::flow::tests::layout();
    let now = after(start, 6_000);
    conversation.prepare(&layout, now, true);

    let scene = conversation.scene(&layout, now, &CommandKeys);

    let keys: Vec<&Span> = scene
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .filter(|span| span.text.contains('⌘'))
        .collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|span| span.ink == Ink::Full));
}

#[test]
fn tints_the_glyph_that_opens_a_line_of_the_conversation() {
    let start = Instant::now();
    let mut conversation = sent("tests", start);
    conversation.colour(colourful());
    conversation.hear(
        ReplyEvent::Text("12 ✓ passent".to_owned()),
        start,
        &IdTranslator,
    );

    let scene = scene_at(&mut conversation, after(start, 5_000));

    let ticked = scene.panes[0]
        .lines
        .iter()
        .flat_map(|placed| &placed.line.spans)
        .find(|span| span.text.contains('✓'))
        .unwrap();
    assert_eq!(ticked.ink, Ink::Rgb([0x4a, 0xde, 0x80]));
}

#[test]
fn keeps_painting_while_a_paced_reply_waits_to_be_shown() {
    let start = Instant::now();
    let mut conversation = sent("hi", start);
    conversation.pace_replies();
    conversation.hear(
        ReplyEvent::Text("un deux trois".to_owned()),
        start,
        &IdTranslator,
    );
    let settled = after(start, 10_000);

    assert!(conversation.is_animating(settled));
    conversation.flush(start, &IdTranslator);
    assert!(!conversation.is_animating(settled));
}

#[test]
fn keeps_painting_for_a_paced_reply_alone_once_everything_else_rests() {
    let start = Instant::now();
    let mut conversation = sent("hi", start);
    conversation.pace_replies();
    conversation.hear(
        ReplyEvent::Text("un deux trois quatre cinq six sept huit".to_owned()),
        start,
        &IdTranslator,
    );
    conversation.flow(after(start, 100), &IdTranslator);

    assert!(conversation.is_animating(after(start, 1_400)));
    conversation.flush(after(start, 1_400), &IdTranslator);
    assert!(!conversation.is_animating(after(start, 3_000)));
}
