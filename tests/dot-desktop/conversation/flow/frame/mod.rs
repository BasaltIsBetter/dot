use dot_api::{ReplyEvent, ToolKind, ToolUse};

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
fn marks_each_tool_in_the_gutter_with_its_pictogram() {
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

    assert_eq!(scene.panes[1].size, TextSize::Small);
    assert_eq!(
        scene.panes[1].lines.iter().map(text_of).collect::<Vec<_>>(),
        vec!["layout.rs"]
    );
    assert!(scene.marks.len() > 3);
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
fn cuts_a_long_tool_label_to_the_gutter() {
    let start = Instant::now();
    let mut conversation = sent("look", start);
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool: ToolUse {
                kind: ToolKind::Run,
                subject: "x".repeat(300),
            },
        },
        start,
        &IdTranslator,
    );

    let scene = scene_at(&mut conversation, after(start, 100));

    let label = text_of(&scene.panes[2].lines[0]);
    assert!(label.ends_with('…'), "{label}");
    let layout = crate::conversation::flow::tests::layout();
    let columns = u16::try_from(label.chars().count()).unwrap();
    let [left, right] = stage().gutter.unwrap();
    let width = f32::from(columns + scene.panes[2].lines[0].line.indent)
        * layout.advance_of(TextSize::Small);
    assert!(width <= right - left + 0.5, "{width}");
    assert!(width > (right - left) / 2.0, "{width}");
}

#[test]
fn draws_a_gutter_pictogram_centred_on_its_row_and_none_on_the_bar() {
    let start = Instant::now();
    let mut conversation = sent("look", start);
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool: ToolUse {
                kind: ToolKind::Read,
                subject: "a.rs".to_owned(),
            },
        },
        start,
        &IdTranslator,
    );

    let scene = scene_at(&mut conversation, after(start, 2_000));

    let label = &scene.panes[1].lines[0];
    let stage = stage();
    let layout = crate::conversation::flow::tests::layout();
    let dot = layout.icon_dot();
    let [left, _] = stage.gutter.unwrap();
    let glyph: Vec<&Rectangle> = scene
        .marks
        .iter()
        .filter(|mark| mark.left >= left - 0.5 && mark.left < left + 9.0 * dot)
        .collect();
    let top = glyph.iter().map(|mark| mark.top).fold(f32::MAX, f32::min);
    let bottom = glyph
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    let label_middle = label.top + layout.line_height_of(TextSize::Small) / 2.0;
    let glyph_middle = top + 7.0 * dot / 2.0;
    assert!(
        (glyph_middle - label_middle).abs() < 3.0,
        "{glyph_middle} {label_middle}"
    );
    assert!((bottom - top - 7.0 * dot).abs() < 0.5);
    assert!(
        glyph
            .iter()
            .all(|mark| mark.top + mark.height <= stage.floor)
    );
}
