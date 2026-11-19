use std::borrow::Cow;
use std::time::Duration;

use dot_api::ReplyEvent;
use winit::dpi::PhysicalSize;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::*;
use crate::canvas::Placed;
use crate::conversation::intent;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

pub(super) struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }

    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
}

pub(super) fn asked(text: &str) -> Prompt {
    Prompt {
        text: text.to_owned(),
        images: Vec::new(),
    }
}

pub(super) fn layout() -> Layout {
    Layout::reflowed(WINDOW, RETINA)
}

pub(super) fn nothing_pasted() -> Option<String> {
    None
}

pub(super) fn type_text(conversation: &mut Conversation, text: &str) {
    conversation.type_key(
        Intent::Edit(Edit::Insert(text.to_owned())),
        &layout(),
        Instant::now(),
        nothing_pasted,
    );
}

pub(super) fn press(
    conversation: &mut Conversation,
    key: NamedKey,
    now: Instant,
) -> Option<Command> {
    let intent = intent(&Key::Named(key), None, ModifiersState::empty()).unwrap();
    conversation.type_key(intent, &layout(), now, nothing_pasted)
}

pub(super) fn press_return(conversation: &mut Conversation, now: Instant) -> Option<Command> {
    press(conversation, NamedKey::Enter, now)
}

pub(super) fn pane_texts(scene: &Scene, pane: usize) -> Vec<String> {
    scene.panes[pane].lines.iter().map(text_of).collect()
}

pub(super) fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

pub(super) fn scene_at(conversation: &Conversation, now: Instant) -> Scene {
    conversation.scene(&layout(), now, true, &IdTranslator)
}

fn transcript_rows(texts: usize) -> Conversation {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    for number in 0..texts {
        type_text(&mut conversation, &format!("prompt {number}"));
        press_return(&mut conversation, now);
        conversation.end_reply(None, &layout(), now);
    }
    conversation
}

pub(super) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(super) fn over_transcript() -> PhysicalPosition<f64> {
    PhysicalPosition::new(1_000.0, 400.0)
}

pub(super) fn over_prompter() -> PhysicalPosition<f64> {
    let zones = Zones::new(&layout(), 1);
    PhysicalPosition::new(1_000.0, f64::from(zones.prompter.top + 1.0))
}

#[test]
fn an_empty_conversation_shows_only_the_prompter() {
    let scene = scene_at(&Conversation::default(), Instant::now());

    assert_eq!(pane_texts(&scene, 0), Vec::<String>::new());
    assert_eq!(
        pane_texts(&scene, 1),
        vec!["› ", "_", "prompter-placeholder"]
    );
    assert!(!scene.animating);
    assert_eq!(scene.lines, Vec::new());
    assert_eq!(scene.strip, None);
}

#[test]
fn return_sends_the_prompt_into_the_transcript_and_asks_claude() {
    let mut conversation = Conversation::default();
    let sent = Instant::now();
    type_text(&mut conversation, "hello");

    let command = press_return(&mut conversation, sent);

    let scene = scene_at(&conversation, sent);
    assert_eq!(command, Some(Command::Ask(asked("hello"))));
    assert_eq!(pane_texts(&scene, 0), vec!["_", "› hello"]);
    assert_eq!(pane_texts(&scene, 1), vec!["› ", "_", "prompter-replying"]);
}

#[test]
fn return_sends_nothing_while_claude_replies() {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    type_text(&mut conversation, "first");
    press_return(&mut conversation, now);
    type_text(&mut conversation, "second");

    let command = press_return(&mut conversation, now);

    assert_eq!(command, None);
    assert_eq!(
        pane_texts(&scene_at(&conversation, now), 1),
        vec!["› second", "_"]
    );
}

#[test]
fn escape_stops_only_a_reply_under_way() {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    let idle = press(&mut conversation, NamedKey::Escape, now);
    type_text(&mut conversation, "hello");
    press_return(&mut conversation, now);

    let replying = press(&mut conversation, NamedKey::Escape, now);

    assert_eq!(idle, None);
    assert_eq!(replying, Some(Command::Stop));
}

#[test]
fn shows_the_thinking_then_the_answer_as_they_arrive() {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    type_text(&mut conversation, "hello");
    press_return(&mut conversation, now);

    conversation.hear(
        ReplyEvent::Thinking("Hm.".into()),
        &layout(),
        now,
        &IdTranslator,
    );
    conversation.hear(ReplyEvent::Text("Hi".into()), &layout(), now, &IdTranslator);
    conversation.hear(
        ReplyEvent::Text(" there".into()),
        &layout(),
        now,
        &IdTranslator,
    );

    let scene = scene_at(&conversation, after(now, 600));
    assert_eq!(pane_texts(&scene, 0), vec!["Hm.", "Hi there_"]);
    assert!(scene.animating);
}

#[test]
fn ends_a_reply_with_its_notice_and_lets_the_next_prompt_go() {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    type_text(&mut conversation, "hello");
    press_return(&mut conversation, now);
    conversation.hear(ReplyEvent::Text("Hi".into()), &layout(), now, &IdTranslator);

    conversation.end_reply(Some("reply-stopped"), &layout(), now);
    type_text(&mut conversation, "again");
    let command = press_return(&mut conversation, now);

    let scene = scene_at(&conversation, now);
    assert_eq!(command, Some(Command::Ask(asked("again"))));
    assert_eq!(
        pane_texts(&scene, 0),
        vec!["› hello", "Hi", "reply-stopped", "_", "› again"]
    );
}

#[test]
fn a_sent_prompt_arrives_orange_where_the_reply_will_be_then_settles() {
    let mut conversation = Conversation::default();
    let sent = Instant::now();
    type_text(&mut conversation, "hello");

    press_return(&mut conversation, sent);

    let arriving = scene_at(&conversation, sent);
    let settled = scene_at(&conversation, after(sent, 1_200));
    assert!(arriving.animating);
    assert_eq!(text_of(&arriving.panes[0].lines[1]), "› hello");
    assert_eq!(arriving.panes[0].lines[1].line.spans[1].heat, u8::MAX);
    assert!(!settled.animating);
    assert_eq!(text_of(&settled.panes[0].lines[1]), "› hello");
    assert_eq!(settled.panes[0].lines[1].line.spans[1].heat, 0);
}

#[test]
fn claude_s_words_take_the_place_of_the_question_then_it_waits_above() {
    let mut conversation = Conversation::default();
    let sent = Instant::now();
    type_text(&mut conversation, "how are you");
    press_return(&mut conversation, sent);
    let written = after(sent, 1_000);

    conversation.hear(
        ReplyEvent::Text("Fine thanks".into()),
        &layout(),
        written,
        &IdTranslator,
    );

    let swapping = scene_at(&conversation, after(written, 150));
    let swapped = pane_texts(&scene_at(&conversation, after(written, 300)), 0);
    conversation.end_reply(None, &layout(), after(written, 300));
    conversation.scroll(
        Wheel::Lines(1.0),
        over_transcript(),
        &layout(),
        after(written, 300),
    );
    let above = pane_texts(&scene_at(&conversation, after(written, 300)), 0);
    assert_eq!(
        pane_texts(&swapping, 0),
        vec!["Fine thanks_", "› ", "how", "are", "you"]
    );
    assert!(
        swapping.panes[0]
            .lines
            .iter()
            .all(|placed| placed.clip.is_some())
    );
    assert!(swapping.animating);
    assert_eq!(swapped, vec!["Fine thanks_"]);
    assert_eq!(above, vec!["› how are you", "Fine thanks"]);
}

#[test]
fn sending_carries_the_question_to_the_top_where_the_reply_will_be_written() {
    let mut conversation = transcript_rows(40);
    let sent = Instant::now();
    type_text(&mut conversation, "latest");

    press_return(&mut conversation, sent);

    let leaving = pane_texts(&scene_at(&conversation, sent), 0);
    let halfway = scene_at(&conversation, after(sent, 300));
    let arrived = scene_at(&conversation, after(sent, 600));
    assert_eq!(leaving[leaving.len() - 2..], ["_", "› latest"]);
    assert_eq!(text_of(halfway.panes[0].lines.last().unwrap()), "› latest");
    assert!(halfway.panes[0].lines.last().unwrap().top > halfway.panes[0].top);
    assert_eq!(pane_texts(&arrived, 0), vec!["_", "› latest"]);
    assert!(
        arrived.panes[0]
            .lines
            .iter()
            .all(|placed| (placed.top - arrived.panes[0].top).abs() < f32::EPSILON)
    );
}

#[test]
fn scrolling_up_from_the_reply_shows_the_prompt_again() {
    let mut conversation = transcript_rows(40);
    let sent = Instant::now();
    type_text(&mut conversation, "latest");
    press_return(&mut conversation, sent);
    conversation.hear(
        ReplyEvent::Text("Hi".into()),
        &layout(),
        after(sent, 600),
        &IdTranslator,
    );

    conversation.scroll(
        Wheel::Lines(1.0),
        over_transcript(),
        &layout(),
        after(sent, 1_000),
    );

    let shown = pane_texts(&scene_at(&conversation, after(sent, 1_000)), 0);
    assert_eq!(shown[shown.len() - 2..], ["› latest", "Hi_"]);
}

#[test]
fn the_scene_animates_while_the_transcript_travels() {
    let mut conversation = Conversation::default();
    let start = Instant::now();
    conversation.transcript.append("hello", start);
    let asked = after(start, 2_000);

    conversation.transcript.begin_reply(30.0, asked);

    assert!(!conversation.transcript.is_fresh(after(asked, 100)));
    assert!(scene_at(&conversation, after(asked, 100)).animating);
    assert!(!scene_at(&conversation, after(asked, 600)).animating);
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

    conversation.scroll(
        Wheel::Lines(2.0),
        over_transcript(),
        &layout(),
        Instant::now(),
    );

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_ne!(after, before);
    assert_eq!(after.last(), Some(&"› prompt 36".to_owned()));
}

#[test]
fn one_wheel_notch_scrolls_three_rows() {
    let mut conversation = transcript_rows(40);
    let now = Instant::now();
    let visible = Zones::new(&layout(), 1)
        .transcript
        .rows(layout().line_height());
    let before = conversation.transcript.window(visible, now).first;

    conversation.scroll(Wheel::Lines(1.0), over_transcript(), &layout(), now);

    let after = conversation.transcript.window(visible, now).first;
    let shown = pane_texts(&scene_at(&conversation, now), 0);
    assert_eq!(before - after, 3);
    assert_eq!(
        shown.first(),
        Some(&format!("› prompt {}", after.div_ceil(2)))
    );
}

#[test]
fn a_shrinking_prompter_hands_the_end_back_to_the_transcript() {
    let mut conversation = transcript_rows(40);
    type_text(&mut conversation, "1\n2\n3\n4\n5\n6\n7\n8");
    conversation.scroll(
        Wheel::Lines(1.0),
        over_transcript(),
        &layout(),
        Instant::now(),
    );
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

    conversation.scroll(
        Wheel::Pixels(4.0 * line),
        over_transcript(),
        &layout(),
        Instant::now(),
    );

    let after = pane_texts(&scene_at(&conversation, Instant::now()), 0);
    assert_eq!(after.last(), Some(&"› prompt 37".to_owned()));
}

#[test]
fn the_wheel_over_the_prompter_leaves_the_transcript_alone() {
    let mut conversation = transcript_rows(40);
    let before = pane_texts(&scene_at(&conversation, Instant::now()), 0);

    conversation.scroll(
        Wheel::Lines(2.0),
        over_prompter(),
        &layout(),
        Instant::now(),
    );
    conversation.scroll(
        Wheel::Lines(2.0),
        PhysicalPosition::new(1_000.0, f64::from(layout().strip_top() + 1.0)),
        &layout(),
        Instant::now(),
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

    conversation.scroll(Wheel::Lines(10.0), inside, &layout(), Instant::now());

    let prompt = pane_texts(&scene_at(&conversation, Instant::now()), 1);
    assert_eq!(prompt.first(), Some(&"› row 1".to_owned()));
}

#[test]
fn sending_from_far_up_the_transcript_starts_the_reply_at_the_top() {
    let mut conversation = transcript_rows(40);
    let sent = Instant::now();
    conversation.scroll(Wheel::Lines(10.0), over_transcript(), &layout(), sent);
    type_text(&mut conversation, "latest");

    press_return(&mut conversation, sent);

    let leaving = pane_texts(&scene_at(&conversation, sent), 0);
    assert_eq!(leaving[leaving.len() - 2..], ["_", "› latest"]);
    assert_eq!(
        pane_texts(&scene_at(&conversation, after(sent, 600)), 0),
        vec!["_", "› latest"]
    );
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

fn png(size: usize) -> Image {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.resize(size, 0);
    Image::from_bytes(bytes).unwrap()
}

#[test]
fn sends_the_images_attached_with_the_text_and_lists_them() {
    let mut conversation = Conversation::default();
    let now = Instant::now();
    conversation.attach("capture.png", png(2_000), &IdTranslator);
    type_text(&mut conversation, "look");

    let command = press_return(&mut conversation, now);

    let Some(Command::Ask(prompt)) = command else {
        panic!("nothing asked");
    };
    assert_eq!(prompt.text, "look");
    assert_eq!(prompt.images, [png(2_000)]);
    assert_eq!(
        pane_texts(&scene_at(&conversation, now), 0),
        vec!["_", "› look", "image-label  capture.png  2 size-kilobytes"]
    );
}

#[test]
fn writes_sizes_in_kilobytes_then_tenths_of_megabytes() {
    assert_eq!(size(1, &IdTranslator), "1 size-kilobytes");
    assert_eq!(size(1_024, &IdTranslator), "1 size-kilobytes");
    assert_eq!(size(1_025, &IdTranslator), "2 size-kilobytes");
    assert_eq!(size(1_048_575, &IdTranslator), "1024 size-kilobytes");
    assert_eq!(size(1_048_576, &IdTranslator), "1.0 size-megabytes");
    assert_eq!(size(1_258_291, &IdTranslator), "1.2 size-megabytes");
    assert_eq!(size(5_242_880, &IdTranslator), "5.0 size-megabytes");
}

#[test]
fn shortens_a_long_image_name_to_the_column() {
    let mut conversation = Conversation::default();
    let long = format!("{}.png", "a".repeat(80));

    conversation.attach(&long, png(10), &IdTranslator);

    let prompter = pane_texts(&scene_at(&conversation, Instant::now()), 1);
    let row = prompter
        .iter()
        .find(|row| row.starts_with("image-label"))
        .unwrap();
    assert!(
        row.chars().count() <= TextWidth::default().columns(),
        "{row}"
    );
    assert!(row.contains('…'));
}

pub(super) fn compact() -> Layout {
    Layout::reflowed(PhysicalSize::new(990, 1800), RETINA)
}

fn compact_scene(conversation: &Conversation) -> Scene {
    conversation.scene(&compact(), Instant::now(), true, &IdTranslator)
}

#[test]
fn fits_the_prompter_and_the_transcript_to_a_compact_column() {
    let mut conversation = Conversation::default();
    let words = "word ".repeat(14);
    type_text(&mut conversation, words.trim_end());
    press_return(&mut conversation, Instant::now());
    conversation.end_reply(None, &layout(), Instant::now());
    type_text(&mut conversation, words.trim_end());

    conversation.fit(&compact());

    let scene = compact_scene(&conversation);
    assert_eq!(compact().columns(), 64);
    assert_eq!(
        pane_texts(&scene, 0),
        vec![format!("› {}", "word ".repeat(12)), "word word".to_owned()]
    );
    assert_eq!(
        pane_texts(&scene, 1)[..2],
        [format!("› {}wo", "word ".repeat(12)), "rd word".to_owned()]
    );
}

#[test]
fn fitting_the_same_width_again_leaves_a_scrolled_prompter_alone() {
    let mut conversation = Conversation::default();
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    type_text(&mut conversation, &text);
    let zones = Zones::new(&layout(), 12);
    let inside = PhysicalPosition::new(1_000.0, f64::from(zones.prompter.top + 1.0));
    conversation.scroll(Wheel::Lines(10.0), inside, &layout(), Instant::now());

    conversation.fit(&layout());

    let prompt = pane_texts(&scene_at(&conversation, Instant::now()), 1);
    assert_eq!(prompt.first(), Some(&"› row 1".to_owned()));
}

#[test]
fn fitting_a_narrower_column_brings_the_caret_back_into_view() {
    let mut conversation = Conversation::default();
    let lines = vec!["x".repeat(70); 16].join("\n");
    type_text(&mut conversation, &lines);
    for _ in 0..8 {
        press(&mut conversation, NamedKey::ArrowUp, Instant::now());
    }
    let wide = pane_texts(&scene_at(&conversation, Instant::now()), 1);

    conversation.fit(&compact());

    let narrow = pane_texts(&compact_scene(&conversation), 1);
    assert!(wide.contains(&"_".to_owned()), "{wide:?}");
    assert!(narrow.contains(&"_".to_owned()), "{narrow:?}");
}

#[test]
fn sends_an_image_row_that_fits_a_compact_column() {
    let mut conversation = Conversation::default();
    conversation.fit(&compact());
    let long = format!("{}.png", "a".repeat(80));
    conversation.attach(&long, png(10), &IdTranslator);

    let sent = Instant::now();
    press_return(&mut conversation, sent);

    let scene = conversation.scene(&compact(), sent, true, &IdTranslator);
    let rows = pane_texts(&scene, 0);
    let image = rows
        .iter()
        .find(|row| row.starts_with("image-label"))
        .unwrap();
    assert_eq!(image.chars().count(), 62, "{image}");
    assert!(image.contains('…'));
}
