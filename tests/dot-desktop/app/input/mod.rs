use std::path::Path;

use dot_api::ReplyEvent;
use winit::dpi::PhysicalPosition;
use winit::keyboard::{ModifiersState, NamedKey};

use super::*;
use crate::app::tests::{
    RETINA, RETINA_SIZE, RecordingCanvas, TestApp, act, app_with, canvas_of, chatting,
    connected_with_status, deliver, flow_texts, named, settled, shown_row, signing_in, typed,
    until_replied,
};
use crate::event::{Delivery, Wheel};
use crate::reply::tests::ScriptedChat;

const ACCOUNT_ROW: usize = 4;
const API_KEY_ROW: usize = 5;
const POINTER: PhysicalPosition<f64> = PhysicalPosition::new(300.4, 200.6);

fn over_row(row: usize) -> PhysicalPosition<f64> {
    let layout = Layout::zoomed(RETINA_SIZE, RETINA);
    PhysicalPosition::new(
        f64::from(layout.x(0)) + 1.0,
        f64::from(layout.line_top(row)) + 1.0,
    )
}

fn pointed() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    act(&mut app, WindowAction::Point(POINTER));
    app
}

fn sent_prompts(count: usize) -> TestApp {
    let (mut app, receiver) = chatting(ScriptedChat::default());
    for number in 0..count {
        act(&mut app, typed(&number.to_string()));
        act(&mut app, named(NamedKey::Enter));
        until_replied(&mut app, &receiver);
    }
    app
}

#[test]
fn a_key_moves_the_selection_on_screen() {
    let mut app = signing_in();

    act(&mut app, named(NamedKey::ArrowDown));
    let account = shown_row(&app, ACCOUNT_ROW);
    act(&mut app, named(NamedKey::ArrowDown));

    assert!(account.starts_with("› sign-in-anthropic-account"));
    assert!(shown_row(&app, API_KEY_ROW).starts_with("› sign-in-api-key"));
}

#[test]
fn composed_text_goes_to_the_key_field_while_signing_in() {
    let mut app = signing_in();
    act(&mut app, named(NamedKey::ArrowDown));
    act(&mut app, named(NamedKey::ArrowDown));
    act(&mut app, named(NamedKey::Enter));

    act(&mut app, WindowAction::Commit("sk".to_owned()));

    assert_eq!(shown_row(&app, 3), "› ••_");
}

#[test]
fn hovering_a_way_selects_it() {
    let mut app = signing_in();

    act(&mut app, WindowAction::Point(over_row(API_KEY_ROW)));

    assert!(shown_row(&app, API_KEY_ROW).starts_with("› "));
}

#[test]
fn clicking_a_way_chooses_it() {
    let mut app = signing_in();
    act(&mut app, WindowAction::Point(over_row(API_KEY_ROW)));

    act(&mut app, WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-key-prompt");
}

#[test]
fn clicking_outside_the_ways_does_nothing() {
    let mut app = signing_in();
    act(&mut app, WindowAction::Point(over_row(0)));

    act(&mut app, WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-intro");
}

#[test]
fn typing_once_connected_writes_in_the_prompter() {
    let mut app = connected_with_status();

    act(&mut app, typed("h"));
    act(&mut app, typed("i"));
    act(&mut app, WindowAction::Commit("é".to_owned()));

    assert_eq!(flow_texts(&app), vec!["› hié"]);
}

#[test]
fn shows_a_composition_in_the_prompter() {
    let mut app = connected_with_status();

    act(&mut app, WindowAction::Compose("^".to_owned()));

    assert_eq!(flow_texts(&app), vec!["› ", "^"]);
}

#[test]
fn ignores_a_composition_while_signing_in() {
    let mut app = app_with(RecordingCanvas::default());
    act(&mut app, WindowAction::Compose("^".to_owned()));
    deliver(
        &mut app,
        Delivery::SignIn(crate::sign_in::Report::Checked(Ok(Some(
            dot_api::Connection::ApiKey,
        )))),
        Instant::now(),
    );

    act(&mut app, WindowAction::Paint);

    assert_eq!(flow_texts(&app), vec!["›", "suggestion-explain-project"]);
}

#[test]
fn return_sends_the_prompt_above_and_waits_for_claude() {
    let (mut app, _receiver) = chatting(ScriptedChat {
        waits_to_be_stopped: true,
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));

    act(&mut app, named(NamedKey::Enter));

    assert_eq!(flow_texts(&app), vec!["› hello", "›"]);
    settled(&mut app);
    assert_eq!(flow_texts(&app), vec!["› hello", "›"]);
}

#[test]
fn the_wheel_scrolls_the_transcript_under_the_pointer() {
    let mut app = sent_prompts(60);
    act(
        &mut app,
        WindowAction::Point(PhysicalPosition::new(1_000.0, 400.0)),
    );
    settled(&mut app);
    let before = flow_texts(&app);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(5.0)));
    settled(&mut app);

    assert_ne!(flow_texts(&app), before);
}

fn over_the_sessions(app: &TestApp) -> PhysicalPosition<f64> {
    let stage = Stage::of(&app.layout(Instant::now()));
    let [left, right] = stage.rail.unwrap();
    PhysicalPosition::new(f64::from(f32::midpoint(left, right)), 400.0)
}

#[test]
fn the_wheel_over_a_long_column_of_sessions_scrolls_it_and_not_the_transcript() {
    let mut app = sent_prompts(60);
    for _ in 0..29 {
        app.open_session();
    }
    app.sessions.switch_to(0);
    let sessions = over_the_sessions(&app);
    act(&mut app, WindowAction::Point(sessions));
    settled(&mut app);
    let before = flow_texts(&app);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(-3.0)));
    let three_down = app.sessions.rail_offset();
    act(&mut app, WindowAction::Scroll(Wheel::Lines(1.0)));
    settled(&mut app);

    assert_eq!(flow_texts(&app), before);
    assert!(three_down > 0.0);
    assert!((app.sessions.rail_offset() - three_down * 2.0 / 3.0).abs() < 0.01);
}

#[test]
fn brings_the_session_shown_into_the_column_when_it_changes() {
    let mut app = sent_prompts(1);
    for _ in 0..29 {
        app.open_session();
    }
    settled(&mut app);
    let at_the_end = app.sessions.rail_offset();

    app.sessions.switch_to(0);
    settled(&mut app);

    assert!(at_the_end > 0.0);
    assert!(app.sessions.rail_offset().abs() < f32::EPSILON);
}

#[test]
fn the_wheel_over_a_short_column_of_sessions_moves_nothing() {
    let mut app = sent_prompts(60);
    let sessions = over_the_sessions(&app);
    act(&mut app, WindowAction::Point(sessions));
    settled(&mut app);
    let before = flow_texts(&app);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(5.0)));
    act(&mut app, WindowAction::Scroll(Wheel::Lines(-5.0)));
    settled(&mut app);

    assert_eq!(flow_texts(&app), before);
    assert!(app.sessions.rail_offset().abs() < f32::EPSILON);
}

#[test]
fn the_wheel_does_nothing_without_a_pointer_or_a_connection() {
    let mut app = sent_prompts(60);
    settled(&mut app);
    let before = flow_texts(&app);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(5.0)));
    settled(&mut app);
    let unpointed = flow_texts(&app);
    let mut signing = signing_in();
    act(
        &mut signing,
        WindowAction::Point(PhysicalPosition::new(1_000.0, 400.0)),
    );
    act(&mut signing, WindowAction::Scroll(Wheel::Lines(5.0)));

    assert_eq!(unpointed, before);
    assert_eq!(canvas_of(&signing).shown.panes, Vec::new());
}

#[test]
fn clicking_in_the_prompt_moves_the_caret() {
    let layout = Layout::reflowed(RETINA_SIZE, RETINA);
    let mut app = connected_with_status();
    act(&mut app, typed("hello"));
    let prompt_top = canvas_of(&app).shown.panes[0]
        .lines
        .iter()
        .find(|placed| crate::app::tests::text_of(&placed.line) == "› hello")
        .map(|placed| placed.top)
        .unwrap();

    act(
        &mut app,
        WindowAction::Point(PhysicalPosition::new(
            f64::from(layout.x(4)) + 1.0,
            f64::from(prompt_top) + 1.0,
        )),
    );
    act(&mut app, WindowAction::Press);
    act(&mut app, typed("X"));

    assert_eq!(flow_texts(&app)[0], "› heXllo");
}

#[test]
fn draws_the_reticle_where_the_pointer_is() {
    let app = pointed();

    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, RETINA)));
}

#[test]
fn hides_the_reticle_when_the_pointer_leaves_the_window() {
    let mut app = pointed();

    act(&mut app, WindowAction::Leave);

    assert_eq!(canvas_of(&app).reticle, None);
}

#[test]
fn hides_the_reticle_while_the_window_is_in_the_background() {
    let mut app = pointed();

    act(&mut app, WindowAction::Blur);
    let blurred = canvas_of(&app).reticle;
    act(&mut app, WindowAction::Focus);

    assert_eq!(blurred, None);
    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, RETINA)));
}

#[test]
fn resizes_the_ring_with_the_screen_scale() {
    let mut app = pointed();

    act(&mut app, WindowAction::Rescale(1.0));

    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, 1.0)));
}

struct DroppedFile(std::path::PathBuf);

impl DroppedFile {
    fn holding(name: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!("dot-drop-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
}

impl Drop for DroppedFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.0) {
            eprintln!("could not remove {}: {error}", self.0.display());
        }
    }
}

fn connected_with_mailbox() -> (TestApp, std::sync::mpsc::Receiver<Delivery>) {
    let (mut app, receiver) = crate::app::tests::app_chatting_with(
        crate::sign_in::FakeConnector::default(),
        ScriptedChat::default(),
    );
    deliver(
        &mut app,
        Delivery::SignIn(crate::sign_in::Report::Checked(Ok(Some(
            dot_api::Connection::ApiKey,
        )))),
        Instant::now(),
    );
    (app, receiver)
}

fn dropped(app: &mut TestApp, receiver: &std::sync::mpsc::Receiver<Delivery>, path: &Path) {
    act(app, WindowAction::Drop(path.to_path_buf()));
    loop {
        let delivery = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let read = matches!(delivery, Delivery::Disk(_));
        deliver(app, delivery, Instant::now());
        if read {
            act(app, WindowAction::Paint);
            return;
        }
    }
}

#[test]
fn a_dropped_image_waits_in_the_prompter() {
    let (mut app, receiver) = connected_with_mailbox();
    let file = DroppedFile::holding("capture.png", b"\x89PNG\r\n\x1a\n pixels");

    dropped(&mut app, &receiver, &file.0);

    let flow = flow_texts(&app);
    let name = file.0.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        flow.contains(&format!("image-label  {name}  1 size-kilobytes")),
        "{flow:?}"
    );
}

#[test]
fn a_dropped_file_that_is_no_image_leaves_a_notice() {
    let (mut app, receiver) = connected_with_mailbox();
    let file = DroppedFile::holding("notes.txt", b"just text");

    dropped(&mut app, &receiver, &file.0);

    assert_eq!(
        flow_texts(&app),
        vec!["image-unsupported", "›", "suggestion-explain-project"]
    );
}

#[test]
fn a_dropped_image_too_large_leaves_a_notice() {
    let (mut app, receiver) = connected_with_mailbox();
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.resize(Image::LARGEST + 1, 0);
    let file = DroppedFile::holding("huge.png", &bytes);

    dropped(&mut app, &receiver, &file.0);

    assert_eq!(
        flow_texts(&app),
        vec!["image-too-large", "›", "suggestion-explain-project"]
    );
}

#[test]
fn a_file_dropped_before_connecting_is_ignored() {
    let mut app = app_with(RecordingCanvas::default());
    let file = DroppedFile::holding("early.png", b"\x89PNG\r\n\x1a\n pixels");

    act(&mut app, WindowAction::Drop(file.0.clone()));
    deliver(
        &mut app,
        Delivery::SignIn(crate::sign_in::Report::Checked(Ok(Some(
            dot_api::Connection::ApiKey,
        )))),
        Instant::now(),
    );
    act(&mut app, WindowAction::Paint);

    assert_eq!(flow_texts(&app), vec!["›", "suggestion-explain-project"]);
}

#[test]
fn command_n_starts_a_new_conversation() {
    let chat = ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bonjour".into())],
        ..ScriptedChat::default()
    };
    let resets = std::sync::Arc::clone(&chat.resets);
    let (mut app, receiver) = chatting(chat);
    act(&mut app, typed("Salut"));
    act(&mut app, named(NamedKey::Enter));
    until_replied(&mut app, &receiver);

    act(&mut app, command("n"));
    settled(&mut app);
    let after_command_n = flow_texts(&app);
    act(&mut app, typed("Nouveau"));
    act(&mut app, named(NamedKey::Enter));
    until_replied(&mut app, &receiver);

    assert!(after_command_n.contains(&"conversation-restarted".to_owned()));
    assert_eq!(resets.load(std::sync::atomic::Ordering::Relaxed), 1);
}

fn command(text: &str) -> WindowAction {
    WindowAction::Key(crate::event::Keystroke {
        key: winit::keyboard::Key::Character(winit::keyboard::SmolStr::new(text)),
        text: Some(winit::keyboard::SmolStr::new(text)),
        modifiers: ModifiersState::SUPER,
    })
}

#[test]
fn the_wheel_moves_down_the_settings_when_it_scrolls_down() {
    assert!((settings_pixels(Wheel::Lines(-1.0)) - 40.0).abs() < f32::EPSILON);
    assert!((settings_pixels(Wheel::Lines(2.0)) + 80.0).abs() < f32::EPSILON);
    assert!((settings_pixels(Wheel::Pixels(-12.0)) - 12.0).abs() < f32::EPSILON);
}
