use std::borrow::Cow;
use std::time::Duration;

use winit::dpi::PhysicalSize;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::*;
use crate::canvas::Placed;
use crate::conversation::intent;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
}

fn layout() -> Layout {
    Layout::new(WINDOW, RETINA)
}

fn nothing_pasted() -> Option<String> {
    None
}

fn type_text(conversation: &mut Conversation, text: &str) {
    conversation.type_key(
        Intent::Edit(Edit::Insert(text.to_owned())),
        &layout(),
        Instant::now(),
        nothing_pasted,
    );
}

fn press_return(conversation: &mut Conversation, now: Instant) {
    let intent = intent(&Key::Named(NamedKey::Enter), None, ModifiersState::empty()).unwrap();
    conversation.type_key(intent, &layout(), now, nothing_pasted);
}

fn pane_texts(scene: &Scene, pane: usize) -> Vec<String> {
    scene.panes[pane].lines.iter().map(text_of).collect()
}

fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn scene_at(conversation: &Conversation, now: Instant) -> Scene {
    conversation.scene(&layout(), now, true, &IdTranslator)
}

fn transcript_rows(texts: usize) -> Conversation {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    for number in 0..texts {
        type_text(&mut conversation, &format!("prompt {number}"));
        press_return(&mut conversation, now);
    }
    conversation
}

fn over_transcript() -> PhysicalPosition<f64> {
    PhysicalPosition::new(1_000.0, 400.0)
}

fn over_prompter() -> PhysicalPosition<f64> {
    let zones = Zones::new(&layout(), 1);
    PhysicalPosition::new(1_000.0, f64::from(zones.prompter.top + 1.0))
}

#[test]
fn an_empty_conversation_shows_only_the_prompter_and_its_rule() {
    let scene = scene_at(&Conversation::default(), Instant::now());

    assert_eq!(pane_texts(&scene, 0), Vec::<String>::new());
    assert_eq!(
        pane_texts(&scene, 1),
        vec!["› ", "_", "prompter-placeholder"]
    );
    assert_eq!(scene.rules.len(), 1);
    assert!(!scene.animating);
    assert_eq!(scene.lines, Vec::new());
    assert_eq!(scene.strip, None);
}

#[test]
fn the_rule_spans_the_column_above_the_prompter() {
    let layout = layout();
    let scene = scene_at(&Conversation::default(), Instant::now());

    let rule = scene.rules[0];
    let zones = Zones::new(&layout, 1);
    assert!((rule.top - zones.rule).abs() < f32::EPSILON);
    assert!((rule.left - layout.x(0).round()).abs() < f32::EPSILON);
    assert!((rule.left + rule.width - layout.x(Layout::COLUMNS).round()).abs() < f32::EPSILON);
    assert_eq!((rule.height, rule.ink), (Layout::HAIRLINE, Ink::Faint));
}

#[test]
fn return_sends_the_prompt_into_the_transcript() {
    let mut conversation = Conversation::default();
    let sent = Instant::now();
    type_text(&mut conversation, "hello");

    press_return(&mut conversation, sent);

    let scene = scene_at(&conversation, sent);
    assert_eq!(pane_texts(&scene, 0), vec!["› hello"]);
    assert_eq!(
        pane_texts(&scene, 1),
        vec!["› ", "_", "prompter-placeholder"]
    );
}

#[test]
fn a_sent_prompt_arrives_orange_then_settles() {
    let mut conversation = Conversation::default();
    let sent = Instant::now();
    type_text(&mut conversation, "hello");
    press_return(&mut conversation, sent);

    let arriving = scene_at(&conversation, sent);
    let settled = scene_at(&conversation, sent + Duration::from_millis(1_200));

    assert!(arriving.animating);
    assert_eq!(arriving.panes[0].lines[0].line.spans[1].heat, u8::MAX);
    assert!(!settled.animating);
    assert_eq!(settled.panes[0].lines[0].line.spans[1].heat, 0);
}

#[test]
fn return_on_an_empty_prompter_sends_nothing() {
    let mut conversation = Conversation::default();

    press_return(&mut conversation, Instant::now());

    assert_eq!(
        pane_texts(&scene_at(&conversation, Instant::now()), 0),
        Vec::<String>::new()
    );
}

#[test]
fn pastes_the_clipboard_at_the_caret() {
    let mut conversation = Conversation::default();

    conversation.type_key(Intent::Paste, &layout(), Instant::now(), || {
        Some("pasted".to_owned())
    });
    conversation.type_key(Intent::Paste, &layout(), Instant::now(), nothing_pasted);

    let scene = scene_at(&conversation, Instant::now());
    assert_eq!(pane_texts(&scene, 1), vec!["› pasted", "_"]);
}

#[test]
fn types_composed_text_and_shows_the_composition() {
    let mut conversation = Conversation::default();

    conversation.compose("^".to_owned());
    let composing = scene_at(&conversation, Instant::now());
    conversation.compose(String::new());
    conversation.commit("ê".to_owned(), &layout());

    assert_eq!(pane_texts(&composing, 1), vec!["› ", "^"]);
    assert_eq!(
        pane_texts(&scene_at(&conversation, Instant::now()), 1),
        vec!["› ê", "_"]
    );
}

#[test]
fn the_prompter_grows_as_rows_are_added() {
    let mut conversation = Conversation::default();

    type_text(&mut conversation, "one\ntwo\nthree");

    let scene = scene_at(&conversation, Instant::now());
    let prompter = &scene.panes[1];
    assert!((prompter.bottom - prompter.top - 3.0 * layout().line_height()).abs() < f32::EPSILON);
    assert_eq!(prompter.lines.len(), 4);
}

#[test]
fn the_wheel_over_the_transcript_scrolls_only_the_transcript() {
    let mut conversation = transcript_rows(40);
    let before = pane_texts(&scene_at(&conversation, Instant::now()), 0);

    conversation.scroll(Wheel::Lines(2.0), over_transcript(), &layout());

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_ne!(after, before);
    assert_eq!(after.last(), Some(&"› prompt 36".to_owned()));
}

#[test]
fn one_wheel_notch_scrolls_three_rows() {
    let mut conversation = transcript_rows(40);

    conversation.scroll(Wheel::Lines(1.0), over_transcript(), &layout());

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_eq!(after.first(), Some(&"› prompt 19".to_owned()));
}

#[test]
fn a_shrinking_prompter_hands_the_end_back_to_the_transcript() {
    let mut conversation = transcript_rows(40);
    type_text(&mut conversation, "1\n2\n3\n4\n5\n6\n7\n8");
    conversation.scroll(Wheel::Lines(1.0), over_transcript(), &layout());
    let backspace = intent(
        &Key::Named(NamedKey::Backspace),
        None,
        ModifiersState::empty(),
    )
    .unwrap();
    for _ in 0..14 {
        conversation.type_key(backspace.clone(), &layout(), Instant::now(), nothing_pasted);
    }

    type_text(&mut conversation, "\n\n\n\n\n\n\n");

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_eq!(after.last(), Some(&"› prompt 39".to_owned()));
}

#[test]
fn tells_the_system_where_the_caret_is_for_accents() {
    let layout = layout();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "hello");

    let caret = conversation.caret_area(&layout);

    let zones = Zones::new(&layout, 1);
    assert!((caret.left - layout.x(7)).abs() < f32::EPSILON);
    assert!((caret.top - zones.prompter.top).abs() < f32::EPSILON);
    assert!((caret.width - layout.advance()).abs() < f32::EPSILON);
    assert!((caret.height - layout.line_height()).abs() < f32::EPSILON);
}

#[test]
fn a_trackpad_scrolls_the_transcript_by_pixels() {
    let mut conversation = transcript_rows(40);
    let line = f64::from(layout().line_height());

    conversation.scroll(Wheel::Pixels(4.0 * line), over_transcript(), &layout());

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_eq!(after.last(), Some(&"› prompt 37".to_owned()));
}

#[test]
fn the_wheel_over_the_prompter_leaves_the_transcript_alone() {
    let mut conversation = transcript_rows(40);
    let before = pane_texts(&scene_at(&conversation, Instant::now()), 0);

    conversation.scroll(Wheel::Lines(2.0), over_prompter(), &layout());
    conversation.scroll(
        Wheel::Lines(2.0),
        PhysicalPosition::new(1_000.0, f64::from(layout().strip_top() + 1.0)),
        &layout(),
    );

    assert_eq!(
        pane_texts(&scene_at(&conversation, Instant::now()), 0),
        before
    );
}

#[test]
fn the_wheel_over_a_long_prompt_scrolls_the_prompt() {
    let mut conversation = Conversation::default();
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    type_text(&mut conversation, &text);
    let zones = Zones::new(&layout(), 12);
    let inside = PhysicalPosition::new(1_000.0, f64::from(zones.prompter.top + 1.0));

    conversation.scroll(Wheel::Lines(10.0), inside, &layout());

    let prompt = pane_texts(&scene_at(&conversation, Instant::now()), 1);
    assert_eq!(prompt.first(), Some(&"› row 1".to_owned()));
}

#[test]
fn sending_brings_a_scrolled_transcript_back_to_the_end() {
    let mut conversation = transcript_rows(40);
    conversation.scroll(Wheel::Lines(10.0), over_transcript(), &layout());

    type_text(&mut conversation, "latest");
    press_return(&mut conversation, Instant::now());

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_eq!(after.last(), Some(&"› latest".to_owned()));
}

#[test]
fn clicking_the_prompter_moves_the_caret_and_clicking_above_does_not() {
    let layout = layout();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "hello");
    let zones = Zones::new(&layout, 1);
    let y = f64::from(zones.prompter.top + 1.0);

    conversation.press(
        PhysicalPosition::new(f64::from(layout.x(4)) + 1.0, y),
        &layout,
    );
    let moved = pane_texts(&scene_at(&conversation, Instant::now()), 1);
    conversation.press(
        PhysicalPosition::new(f64::from(layout.x(6)) + 1.0, 400.0),
        &layout,
    );
    let unmoved = scene_at(&conversation, Instant::now());

    assert_eq!(moved, vec!["› hello", "_"]);
    assert_eq!(unmoved.panes[1].lines[1].line.indent, 4);
}
