use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use super::*;
use crate::conversation::flow::tests::{
    IdTranslator, after, compact, layout, press, scene_at, type_text,
};

fn prompter_shift(conversation: &Conversation, now: Instant) -> f32 {
    conversation
        .scene(&layout(), now, true, &IdTranslator)
        .panes[1]
        .shift
}

fn centred(content_columns: u16) -> f32 {
    let field = content_columns.next_multiple_of(16).min(96);
    (f32::from(96 - field) / 2.0 * layout().advance()).round()
}

#[test]
fn starts_with_the_empty_prompter_centred_and_still() {
    let mut conversation = Conversation::default();
    let now = Instant::now();

    conversation.prepare(&layout(), now, &IdTranslator);

    let scene = scene_at(&conversation, now);
    assert!((scene.panes[1].shift - centred(4 + 20)).abs() < f32::EPSILON);
    assert!(scene.panes[0].shift.abs() < f32::EPSILON);
    assert!(!scene.animating);
}

#[test]
fn widens_by_a_step_past_its_edge_then_narrows_back() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    type_text(&mut conversation, "hello world!");
    conversation.prepare(&layout(), start, &IdTranslator);
    type_text(&mut conversation, "!!");
    conversation.prepare(&layout(), start, &IdTranslator);
    let wider = prompter_shift(&conversation, after(start, 300));

    press(&mut conversation, NamedKey::Backspace, start);
    conversation.prepare(&layout(), after(start, 300), &IdTranslator);

    assert!((wider - centred(32)).abs() < f32::EPSILON);
    let narrowing = prompter_shift(&conversation, after(start, 450));
    assert!(narrowing > wider && narrowing < centred(16), "{narrowing}");
    assert!((prompter_shift(&conversation, after(start, 600)) - centred(16)).abs() < f32::EPSILON);
}

#[test]
fn keeps_the_text_still_while_it_stays_within_its_step() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    type_text(&mut conversation, "hello");
    conversation.prepare(&layout(), start, &IdTranslator);
    let before = prompter_shift(&conversation, after(start, 300));

    type_text(&mut conversation, " you");
    conversation.prepare(&layout(), after(start, 300), &IdTranslator);

    assert!((before - centred(16)).abs() < f32::EPSILON);
    assert!((prompter_shift(&conversation, after(start, 300)) - before).abs() < f32::EPSILON);
    assert!(!scene_at(&conversation, after(start, 300)).animating);
}

#[test]
fn slides_toward_its_new_place_and_animates_meanwhile() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    conversation.prepare(&layout(), start, &IdTranslator);
    type_text(&mut conversation, "hello");

    conversation.prepare(&layout(), start, &IdTranslator);

    let halfway = prompter_shift(&conversation, after(start, 150));
    assert!(halfway > centred(24) && halfway < centred(8), "{halfway}");
    assert!(scene_at(&conversation, after(start, 150)).animating);
    assert!(!scene_at(&conversation, after(start, 300)).animating);
}

#[test]
fn fills_the_whole_column_once_the_prompt_wraps() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    type_text(&mut conversation, "first\nsecond");

    conversation.prepare(&layout(), start, &IdTranslator);

    assert!(prompter_shift(&conversation, start).abs() < f32::EPSILON);
}

#[test]
fn jumps_to_its_place_when_the_column_changes_width() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    conversation.prepare(&layout(), start, &IdTranslator);
    type_text(&mut conversation, "hi");

    conversation.prepare(&compact(), start, &IdTranslator);

    let scene = conversation.scene(&compact(), start, true, &IdTranslator);
    let expected = (f32::from(64_u16 - 16) / 2.0 * compact().advance()).round();
    assert!((scene.panes[1].shift - expected).abs() < f32::EPSILON);
    assert!(!scene.animating);
}

#[test]
fn clicks_and_reports_the_caret_where_the_centred_text_is() {
    let layout = layout();
    let mut conversation = Conversation::default();
    let start = Instant::now();
    type_text(&mut conversation, "hello");
    conversation.prepare(&layout, start, &IdTranslator);
    let zones = Zones::new(&layout, 1);
    let y = f64::from(zones.prompter.top + 1.0);

    conversation.press(
        PhysicalPosition::new(f64::from(layout.x(4) + centred(8)) + 1.0, y),
        &layout,
    );

    let caret = conversation.caret_area(&layout);
    assert!((caret.left - (layout.x(4) + centred(8))).abs() < f32::EPSILON);
}
