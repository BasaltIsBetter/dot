use std::time::Duration;

use dot_api::ReplyEvent;
use winit::dpi::PhysicalSize;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::*;
pub(super) use crate::app::tests::IdTranslator;
use crate::canvas::Scene;
use crate::conversation::intent;
use crate::conversation::prompter::Edit;
use dot_transcript::{OPAQUE, Placed};

pub(super) const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
pub(super) const NARROW: PhysicalSize<u32> = PhysicalSize::new(1000, 700);

pub(super) fn asked(text: &str) -> Prompt {
    Prompt {
        text: text.to_owned(),
        images: Vec::new(),
    }
}

pub(super) fn layout() -> Layout {
    Layout::reflowed(WINDOW, RETINA)
}

pub(super) fn stage() -> Stage {
    Stage::of(&layout())
}

pub(super) fn nothing_pasted() -> Option<String> {
    None
}

pub(super) fn type_text(conversation: &mut Conversation, text: &str, now: Instant) {
    conversation.type_key(
        Intent::Edit(Edit::Insert(text.to_owned())),
        now,
        nothing_pasted,
        &IdTranslator,
    );
}

pub(super) fn press(
    conversation: &mut Conversation,
    key: NamedKey,
    now: Instant,
) -> Option<Command> {
    let intent = intent(&Key::Named(key), None, ModifiersState::empty()).unwrap();
    conversation.type_key(intent, now, nothing_pasted, &IdTranslator)
}

pub(super) fn press_return(conversation: &mut Conversation, now: Instant) -> Option<Command> {
    press(conversation, NamedKey::Enter, now)
}

pub(super) fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

pub(super) fn scene_at(conversation: &mut Conversation, now: Instant) -> Scene {
    conversation.prepare(&layout(), now, true);
    conversation.scene(&layout(), now, &IdTranslator)
}

pub(super) fn flow_texts(scene: &Scene) -> Vec<String> {
    scene.panes[0].lines.iter().map(text_of).collect()
}

pub(super) fn line_top() -> f32 {
    let stage = stage();
    (stage.centre - stage.row / 2.0).round()
}

pub(super) fn on_the_line(scene: &Scene) -> Vec<String> {
    let top = line_top();
    scene.panes[0]
        .lines
        .iter()
        .filter(|placed| (placed.top - top).abs() < 1.0)
        .map(text_of)
        .collect()
}

pub(super) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(super) fn sent(text: &str, start: Instant) -> Conversation {
    let mut conversation = Conversation::default();
    type_text(&mut conversation, text, start);
    press_return(&mut conversation, start);
    conversation
}

pub(super) fn answered(exchanges: usize, start: Instant) -> Conversation {
    let mut conversation = Conversation::default();
    for number in 0..exchanges {
        type_text(&mut conversation, &format!("prompt {number}"), start);
        press_return(&mut conversation, start);
        let reply = ReplyEvent::Text(format!("reply {number} goes on for a while"));
        conversation.hear(reply, start, &IdTranslator);
        conversation.end_reply(None, Some(12), start);
    }
    conversation
}

#[test]
fn an_empty_conversation_writes_on_the_reading_line() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    let scene = scene_at(&mut conversation, start);

    assert_eq!(
        on_the_line(&scene),
        vec!["›", "suggestion-explain-project", "_"]
    );
}

#[test]
fn typing_writes_on_the_reading_line_from_the_marker() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    type_text(&mut conversation, "hello", start);

    let scene = scene_at(&mut conversation, after(start, 600));
    assert_eq!(on_the_line(&scene), vec!["› hello", "_"]);
    let first = &scene.panes[0].lines[0];
    assert_eq!(first.line.indent, 0);
    assert!(scene.panes[0].left > 0.0);
}

#[test]
fn what_is_typed_arrives_orange_and_cools() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "hi", start);

    let typing = scene_at(&mut conversation, start);
    let cooled = scene_at(&mut conversation, after(start, 450));

    let heat_of = |scene: &Scene| scene.panes[0].lines[0].line.spans[1].heat;
    assert_eq!(heat_of(&typing), u8::MAX);
    assert_eq!(heat_of(&cooled), 0);
    assert!(typing.animating);
}

#[test]
fn sending_keeps_the_question_on_the_reading_line() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "how are you", start);

    let command = press_return(&mut conversation, start);

    assert_eq!(command, Some(Command::Ask(asked("how are you"))));
    let scene = scene_at(&mut conversation, after(start, 2_000));
    assert_eq!(on_the_line(&scene), vec!["› how are you"]);
}

#[test]
fn sends_nothing_when_nothing_is_written_or_while_claude_replies() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    let empty = press_return(&mut conversation, start);
    let mut replying = sent("first", start);
    type_text(&mut replying, "second", start);

    let while_replying = press_return(&mut replying, start);

    assert_eq!(empty, None);
    assert_eq!(while_replying, None);
}

#[test]
fn the_reply_replaces_the_question_on_the_reading_line() {
    let start = Instant::now();
    let mut conversation = sent("how are you", start);

    conversation.hear(
        ReplyEvent::Text("Fine thanks, and you?".to_owned()),
        after(start, 100),
        &IdTranslator,
    );

    let scene = scene_at(&mut conversation, after(start, 2_000));
    assert_eq!(on_the_line(&scene), vec!["Fine thanks, and you?_"]);
    assert!(flow_texts(&scene).contains(&"› how are you".to_owned()));
}

#[test]
fn a_long_reply_scrolls_once_the_prompt_below_it_reaches_the_bottom() {
    let start = Instant::now();
    let mut conversation = sent("list", start);
    let rows_below = stage().rows_below();

    let reply = "row\n".repeat(40);
    conversation.hear(ReplyEvent::Text(reply), after(start, 100), &IdTranslator);

    let scene = scene_at(&mut conversation, after(start, 2_000));
    let stage = stage();
    let lowest = scene.panes[0]
        .lines
        .iter()
        .filter(|placed| text_of(placed) == "›")
        .map(|placed| placed.top)
        .next()
        .unwrap();
    let below_the_line = ((lowest - line_top()) / stage.row).round();
    assert!((below_the_line - f32::from(u16::try_from(rows_below).unwrap())).abs() < 1.0);
}

#[test]
fn the_line_comes_back_to_the_prompt_once_the_reply_ends() {
    let start = Instant::now();
    let mut conversation = sent("list", start);
    conversation.hear(
        ReplyEvent::Text("a\nb\nc\nd".to_owned()),
        after(start, 100),
        &IdTranslator,
    );
    scene_at(&mut conversation, after(start, 200));

    conversation.end_reply(None, None, after(start, 300));
    scene_at(&mut conversation, after(start, 300));

    let scene = scene_at(&mut conversation, after(start, 200 + 1_060));
    assert_eq!(
        on_the_line(&scene),
        vec!["›", "suggestion-explain-project", "_"]
    );
}

#[test]
fn a_reader_looking_back_stays_where_it_reads_while_text_arrives() {
    let start = Instant::now();
    let mut conversation = answered(3, start);
    type_text(&mut conversation, "more", start);
    press_return(&mut conversation, start);
    scene_at(&mut conversation, start);

    conversation.scroll(
        Wheel::Lines(2.0),
        (centre(), &layout()),
        start,
        &IdTranslator,
    );
    scene_at(&mut conversation, start);
    let read = on_the_line(&scene_at(&mut conversation, after(start, 400)));
    conversation.hear(
        ReplyEvent::Text("a\nb\nc".to_owned()),
        after(start, 500),
        &IdTranslator,
    );
    scene_at(&mut conversation, after(start, 500));
    let still = on_the_line(&scene_at(&mut conversation, after(start, 900)));

    assert_ne!(read, Vec::<String>::new());
    assert_eq!(still, read);
}

#[test]
fn typing_comes_back_to_the_reading_line() {
    let start = Instant::now();
    let mut conversation = answered(3, start);
    scene_at(&mut conversation, start);
    conversation.scroll(
        Wheel::Lines(2.0),
        (centre(), &layout()),
        start,
        &IdTranslator,
    );
    scene_at(&mut conversation, start);
    scene_at(&mut conversation, after(start, 400));

    type_text(&mut conversation, "x", after(start, 400));
    scene_at(&mut conversation, after(start, 400));

    let scene = scene_at(&mut conversation, after(start, 400 + 1_060));
    assert_eq!(on_the_line(&scene), vec!["› x", "_"]);
}

#[test]
fn the_wheel_over_the_status_bar_scrolls_nothing() {
    let start = Instant::now();
    let mut conversation = answered(3, start);
    let before = on_the_line(&scene_at(&mut conversation, start));
    let bar = PhysicalPosition::new(1_000.0, f64::from(stage().floor + 2.0));

    conversation.scroll(Wheel::Lines(2.0), (bar, &layout()), start, &IdTranslator);

    assert_eq!(
        on_the_line(&scene_at(&mut conversation, after(start, 400))),
        before
    );
}

#[test]
fn older_exchanges_recede_and_the_latest_reads_in_full_ink() {
    let start = Instant::now();
    let mut conversation = answered(2, start);

    let scene = scene_at(&mut conversation, after(start, 3_000));

    let alpha_of = |text: &str| {
        scene.panes[0]
            .lines
            .iter()
            .find(|placed| text_of(placed) == text)
            .map(|placed| placed.line.alpha)
    };
    assert_eq!(alpha_of("› prompt 0"), Some(163));
    assert_eq!(alpha_of("reply 0 goes on for a while"), Some(163));
    assert_eq!(alpha_of("› prompt 1"), Some(OPAQUE));
    assert_eq!(alpha_of("reply 1 goes on for a while"), Some(OPAQUE));
}

#[test]
fn a_click_on_the_prompt_moves_the_caret_there() {
    let start = Instant::now();
    let mut conversation = Conversation::default();
    type_text(&mut conversation, "hello", start);
    scene_at(&mut conversation, start);
    let layout = layout();
    let at = PhysicalPosition::new(f64::from(layout.x(4) + 1.0), f64::from(line_top() + 2.0));

    conversation.press(at, &layout, start);
    type_text(&mut conversation, "X", start);

    let scene = scene_at(&mut conversation, after(start, 600));
    assert_eq!(on_the_line(&scene)[0], "› heXllo");
}

#[test]
fn a_click_on_the_prompt_under_a_conversation_moves_the_caret_there() {
    let start = Instant::now();
    let mut conversation = answered(2, start);
    type_text(&mut conversation, "hello", start);
    scene_at(&mut conversation, after(start, 2_000));
    let layout = layout();
    let at = PhysicalPosition::new(f64::from(layout.x(4) + 1.0), f64::from(line_top() + 2.0));

    conversation.press(at, &layout, after(start, 2_000));
    type_text(&mut conversation, "X", after(start, 2_000));

    let scene = scene_at(&mut conversation, after(start, 2_600));
    assert_eq!(on_the_line(&scene)[0], "› heXllo");
}

#[test]
fn a_click_on_the_conversation_above_changes_nothing() {
    let start = Instant::now();
    let mut conversation = answered(2, start);
    type_text(&mut conversation, "draft", start);
    scene_at(&mut conversation, start);
    let layout = layout();

    conversation.press(
        PhysicalPosition::new(f64::from(layout.x(4)), 50.0),
        &layout,
        start,
    );
    type_text(&mut conversation, "!", start);

    let scene = scene_at(&mut conversation, after(start, 600));
    assert_eq!(on_the_line(&scene)[0], "› draft!");
}

#[test]
fn rewraps_everything_when_the_column_changes_width() {
    let start = Instant::now();
    let mut conversation = sent(&"word ".repeat(40), start);
    scene_at(&mut conversation, start);
    let narrow = Layout::reflowed(NARROW, RETINA);

    let rewrapped = conversation.fit(&narrow, start);
    let again = conversation.fit(&narrow, start);

    assert!(rewrapped);
    assert!(!again);
}

#[test]
fn stop_and_start_over_are_asked_only_when_they_apply() {
    let start = Instant::now();
    let mut idle = Conversation::default();
    let mut replying = sent("hi", start);

    assert_eq!(press(&mut idle, NamedKey::Escape, start), None);
    assert_eq!(
        press(&mut replying, NamedKey::Escape, start),
        Some(Command::Stop)
    );
    assert_eq!(
        idle.type_key(Intent::StartOver, start, nothing_pasted, &IdTranslator),
        Some(Command::StartOver)
    );
    assert_eq!(
        replying.type_key(Intent::StartOver, start, nothing_pasted, &IdTranslator),
        None
    );
}

#[test]
fn pastes_the_clipboard_into_the_prompt() {
    let start = Instant::now();
    let mut conversation = Conversation::default();

    conversation.type_key(
        Intent::Paste,
        start,
        || Some("pasted".to_owned()),
        &IdTranslator,
    );
    conversation.type_key(Intent::Paste, start, nothing_pasted, &IdTranslator);

    let scene = scene_at(&mut conversation, after(start, 600));
    assert_eq!(on_the_line(&scene)[0], "› pasted");
}

#[test]
fn names_the_size_of_an_image_in_kibibytes_then_tenths_of_mebibytes() {
    assert_eq!(size(1, &IdTranslator), "1 size-kilobytes");
    assert_eq!(size(1_024, &IdTranslator), "1 size-kilobytes");
    assert_eq!(size(1_025, &IdTranslator), "2 size-kilobytes");
    assert_eq!(size(1_048_576, &IdTranslator), "1.0 size-megabytes");
    assert_eq!(size(1_258_291, &IdTranslator), "1.2 size-megabytes");
}

pub(super) fn centre() -> PhysicalPosition<f64> {
    PhysicalPosition::new(1_000.0, f64::from(stage().centre))
}

#[test]
fn the_trackpad_reads_back_by_rows_of_its_pixels() {
    let start = Instant::now();
    let mut conversation = answered(4, start);
    scene_at(&mut conversation, start);
    let before = conversation.reader.focus(start);
    let row = f64::from(stage().row);

    conversation.scroll(
        Wheel::Pixels(row * 2.0),
        (centre(), &layout()),
        start,
        &IdTranslator,
    );
    scene_at(&mut conversation, start);
    scene_at(&mut conversation, after(start, 1_000));

    let moved = before - conversation.reader.focus(after(start, 1_000));
    assert!((moved - 2.0).abs() < 0.01, "{moved}");
}

#[test]
fn lights_no_caret_at_the_end_of_the_reply_while_a_prompt_is_typed() {
    let start = Instant::now();
    let mut conversation = sent("look", start);
    conversation.hear(ReplyEvent::Text("Hello".into()), start, &IdTranslator);
    type_text(&mut conversation, "next", after(start, 100));

    let scene = scene_at(&mut conversation, after(start, 2_000));

    let orange_carets = scene.panes[0]
        .lines
        .iter()
        .flat_map(|placed| &placed.line.spans)
        .filter(|span| span.text == "_" && span.ink == dot_transcript::Ink::Live)
        .count();
    assert_eq!(orange_carets, 0);
}
