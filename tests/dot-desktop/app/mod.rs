use std::borrow::Cow;
use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Connection, Language, Text, Translator};
use winit::keyboard::{Key, NamedKey, SmolStr};

use super::*;
use crate::canvas::{Bounds, Reticle, Scene};
use crate::editor::tests::MemoryWorkspace;
use crate::event::Keystroke;
use crate::reply::tests::ScriptedChat;
use crate::settings::tests::{MemoryPreferences, test_settings};
use crate::sign_in::{FakeConnector, Report};
use crate::status_bar::FixedSession;
use dot_transcript::Line;

pub(crate) const RETINA_SIZE: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
pub(crate) const RETINA: f64 = 2.0;
const FULL_SIZE_TEXT: f32 = 13.0 * 2.0 * 0.8;
const PATIENCE: Duration = Duration::from_secs(5);
const EVERY_MOTION: Duration = Duration::from_secs(2);
const PACED_FRAME: Duration = Duration::from_millis(100);
const PACED_FRAMES: u32 = 200;

thread_local! {
    static LANGUAGE_USED: Cell<Option<Language>> = const { Cell::new(None) };
}

pub(crate) struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }
    fn use_language(&self, language: Language) {
        LANGUAGE_USED.set(Some(language));
    }
}

pub(crate) fn language_used() -> Option<Language> {
    LANGUAGE_USED.get()
}

#[derive(Default)]
pub(crate) struct RecordingCanvas {
    pub(crate) resized_to: Option<PhysicalSize<u32>>,
    pub(crate) shown: Scene,
    pub(crate) reticle: Option<Reticle>,
    pub(crate) composition: Option<Bounds>,
    pub(crate) paint_count: u32,
    pub(crate) frames_requested: u32,
    pub(crate) fails_to_paint: bool,
}

impl Canvas for RecordingCanvas {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.resized_to = Some(size);
    }

    fn show(&mut self, scene: Scene) {
        self.shown = scene;
    }

    fn point(&mut self, reticle: Option<Reticle>) {
        self.reticle = reticle;
    }

    fn allow_composition(&mut self, caret: Option<Bounds>) {
        self.composition = caret;
    }

    fn request_frame(&mut self) {
        self.frames_requested += 1;
    }

    fn paint(&mut self) -> Result<(), DesktopError> {
        self.paint_count += 1;
        if self.fails_to_paint {
            return Err(DesktopError::SurfaceLost);
        }
        Ok(())
    }
}

pub(crate) struct TestParts;

impl Parts for TestParts {
    type Translator = IdTranslator;
    type Connector = FakeConnector;
    type Session = FixedSession;
    type Chat = ScriptedChat;
    type Workspace = MemoryWorkspace;
    type Preferences = MemoryPreferences;
    type Mailbox = Sender<Delivery>;
    type Canvas = RecordingCanvas;
}

pub(crate) type TestApp = DesktopApp<'static, TestParts>;

pub(crate) fn opener(first: ScriptedChat) -> Opener<TestParts> {
    let mut first = Some(first);
    Box::new(move |_folder| (FixedSession::default(), first.take().unwrap_or_default()))
}

pub(crate) fn choices_with(preferences: MemoryPreferences) -> Choices<MemoryPreferences> {
    Choices {
        preferences,
        settings: test_settings(),
        defaults: test_settings(),
        folder: None,
    }
}

pub(crate) fn app_and_mailbox() -> (TestApp, Receiver<Delivery>) {
    app_chatting_with(FakeConnector::default(), ScriptedChat::default())
}

pub(crate) fn app_chatting_with(
    connector: FakeConnector,
    chat: ScriptedChat,
) -> (TestApp, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let mut app = DesktopApp::new(
        &IdTranslator,
        connector,
        opener(chat),
        choices_with(MemoryPreferences::default()),
        (sender, Arc::new(MemoryWorkspace::default())),
    );
    app.canvas = Some(RecordingCanvas::default());
    app.window.size = RETINA_SIZE;
    app.window.scale_factor = RETINA;
    app.window.focused = true;
    (app, receiver)
}

pub(crate) fn app_with(canvas: RecordingCanvas) -> TestApp {
    let (mut app, _receiver) = app_and_mailbox();
    app.canvas = Some(canvas);
    app
}

pub(crate) fn act(app: &mut TestApp, action: WindowAction) -> Flow {
    let now = Instant::now();
    let flow = app.handle(action, now);
    show_requested_frame(app, now);
    flow
}

pub(crate) fn deliver(app: &mut TestApp, delivery: Delivery, now: Instant) {
    app.deliver(delivery, now);
    show_requested_frame(app, now);
}

fn show_requested_frame(app: &mut TestApp, now: Instant) {
    let requested = app
        .canvas
        .as_mut()
        .map(|canvas| std::mem::take(&mut canvas.frames_requested));
    if requested.is_some_and(|requested| requested > 0) {
        app.compose(now);
    }
}

pub(crate) fn settled(app: &mut TestApp) {
    let later = Instant::now() + EVERY_MOTION;
    app.handle(WindowAction::Paint, later);
    for frame in 1..=PACED_FRAMES {
        if !app.sessions.are_pacing() {
            return;
        }
        app.handle(WindowAction::Paint, later + PACED_FRAME * frame);
    }
}

pub(crate) fn connected_with_status() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );
    deliver(
        &mut app,
        Delivery::Session(FixedSession::default().0),
        Instant::now(),
    );
    app
}

pub(crate) fn chatting(chat: ScriptedChat) -> (TestApp, Receiver<Delivery>) {
    let (mut app, receiver) = app_chatting_with(crate::reply::tests::connected(), chat);
    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );
    deliver(
        &mut app,
        Delivery::Session(FixedSession::default().0),
        Instant::now(),
    );
    (app, receiver)
}

pub(crate) fn until_replied(app: &mut TestApp, receiver: &Receiver<Delivery>) {
    while flow_texts(app).last().map(String::as_str) != Some("suggestion-explain-project") {
        let delivery = receiver.recv_timeout(PATIENCE).unwrap();
        deliver(app, delivery, Instant::now());
        settled(app);
    }
    settled(app);
}

pub(crate) fn signing_in() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    app.sign_in.report(Report::Checked(Ok(None)));
    app
}

pub(crate) fn canvas_of(app: &TestApp) -> &RecordingCanvas {
    app.canvas.as_ref().unwrap()
}

pub(crate) fn shown_row(app: &TestApp, row: usize) -> String {
    text_of(&canvas_of(app).shown.lines[row])
}

pub(crate) fn text_of(line: &Line) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

pub(crate) fn pane_texts(app: &TestApp, pane: usize) -> Vec<String> {
    canvas_of(app).shown.panes[pane]
        .lines
        .iter()
        .map(|placed| text_of(&placed.line))
        .collect()
}

pub(crate) fn flow_texts(app: &TestApp) -> Vec<String> {
    let mut texts = pane_texts(app, 0);
    texts.retain(|text| text != "_");
    texts
}

pub(crate) fn keystroke(key: Key, text: Option<&str>) -> WindowAction {
    WindowAction::Key(Keystroke {
        key,
        text: text.map(SmolStr::new),
        modifiers: ModifiersState::empty(),
    })
}

pub(crate) fn named(key: NamedKey) -> WindowAction {
    keystroke(Key::Named(key), None)
}

pub(crate) fn typed(text: &str) -> WindowAction {
    keystroke(Key::Character(SmolStr::new(text)), Some(text))
}

pub(crate) fn shown_strip(app: &TestApp) -> String {
    text_of(&canvas_of(app).shown.strip.as_ref().unwrap().text)
}

#[test]
fn closing_the_window_quits_without_error() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(act(&mut app, WindowAction::Close), Flow::Exit);
    assert!(app.into_result().is_ok());
}

#[test]
fn resizing_the_window_resizes_the_canvas() {
    let mut app = app_with(RecordingCanvas::default());
    let smaller = PhysicalSize::new(1800, 1200);

    assert_eq!(act(&mut app, WindowAction::Resize(smaller)), Flow::Continue);
    assert_eq!(canvas_of(&app).resized_to, Some(smaller));
    assert_eq!(app.window.size, smaller);
}

#[test]
fn a_redraw_request_paints_the_canvas_once() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(act(&mut app, WindowAction::Paint), Flow::Continue);
    assert_eq!(canvas_of(&app).paint_count, 1);
}

#[test]
fn a_failed_paint_stops_dot_with_its_error() {
    let mut app = app_with(RecordingCanvas {
        fails_to_paint: true,
        ..RecordingCanvas::default()
    });

    assert_eq!(act(&mut app, WindowAction::Paint), Flow::Exit);
    assert!(matches!(app.into_result(), Err(DesktopError::SurfaceLost)));
}

#[test]
fn other_events_leave_the_canvas_untouched() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(act(&mut app, WindowAction::Ignore), Flow::Continue);
    assert_eq!(canvas_of(&app).paint_count, 0);
    assert_eq!(canvas_of(&app).shown, Scene::default());
}

#[test]
fn events_before_the_window_opens_change_nothing() {
    let (sender, _receiver) = mpsc::channel();
    let mut app: TestApp = DesktopApp::new(
        &IdTranslator,
        FakeConnector::default(),
        opener(ScriptedChat::default()),
        choices_with(MemoryPreferences::default()),
        (sender, Arc::new(MemoryWorkspace::default())),
    );

    assert_eq!(act(&mut app, WindowAction::Paint), Flow::Continue);
    assert_eq!(act(&mut app, named(NamedKey::ArrowDown)), Flow::Continue);
    assert!(app.into_result().is_ok());
}

#[test]
fn remembers_the_modifiers_and_the_scale() {
    let mut app = app_with(RecordingCanvas::default());

    act(&mut app, WindowAction::Modifiers(ModifiersState::SUPER));
    act(&mut app, WindowAction::Rescale(1.0));

    assert_eq!(app.window.modifiers, ModifiersState::SUPER);
    assert!((app.window.scale_factor - 1.0).abs() < f64::EPSILON);
}

#[test]
fn a_connected_dot_shows_the_prompter_instead_of_the_sign_in_screen() {
    let mut app = app_with(RecordingCanvas::default());
    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );

    act(&mut app, WindowAction::Paint);

    let shown = &canvas_of(&app).shown;
    assert_eq!(shown.lines, Vec::<Line>::new());
    assert_eq!(shown.strip, None);
    assert_eq!(flow_texts(&app), vec!["›", "suggestion-explain-project"]);
}

#[test]
fn takes_composed_text_from_the_system_only_once_connected() {
    let mut app = app_with(RecordingCanvas::default());
    act(&mut app, WindowAction::Paint);
    let while_signing_in = canvas_of(&app).composition;

    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );
    act(&mut app, WindowAction::Paint);

    assert_eq!(app.screen, Screen::Chat);
    let conversation = &app.sessions.active().conversation;
    assert_eq!(while_signing_in, None);
    assert_eq!(
        canvas_of(&app).composition,
        Some(conversation.caret_area(&app.layout(Instant::now()), Instant::now()))
    );
}

#[test]
fn painting_follows_the_clock_of_a_sent_prompt() {
    let mut app = connected_with_status();
    act(&mut app, typed("hi"));
    let sent = Instant::now();
    app.handle(named(NamedKey::Enter), sent);

    app.handle(WindowAction::Paint, sent);
    let arriving = canvas_of(&app).shown.animating;
    app.handle(WindowAction::Paint, sent + Duration::from_secs(2));

    assert!(arriving);
    assert!(!canvas_of(&app).shown.animating);
    assert_eq!(canvas_of(&app).paint_count, 2);
}

#[test]
fn shows_the_status_bar_once_connected_and_the_session_read() {
    let app = connected_with_status();

    let strip = shown_strip(&app);
    assert!(
        strip.trim_start().starts_with("Opus 5.5 ◇ effort-high"),
        "{strip}"
    );
    assert!(strip.contains(" Dot feat/mac-app +0/−0 "), "{strip}");
    assert!(
        strip.ends_with(" cost-dollars 0decimal-separator00 ◇"),
        "{strip}"
    );
    assert_eq!(canvas_of(&app).shown.lines, Vec::<Line>::new());
}

#[test]
fn hides_the_status_bar_while_signing_in() {
    let mut app = signing_in();

    deliver(
        &mut app,
        Delivery::Session(FixedSession::default().0),
        Instant::now(),
    );

    assert_eq!(canvas_of(&app).shown.strip, None);
    assert_ne!(canvas_of(&app).shown.lines, Vec::<Line>::new());
    assert_eq!(canvas_of(&app).shown.panes, Vec::new());
}

#[test]
fn fits_the_status_bar_again_when_the_window_resizes() {
    let mut app = connected_with_status();
    let wide = shown_strip(&app);

    act(&mut app, WindowAction::Resize(PhysicalSize::new(960, 640)));

    let narrow = shown_strip(&app);
    assert!(narrow.chars().count() < wide.chars().count());
    assert!(narrow.trim_start().starts_with("Opus 5.5"));
}

#[test]
fn keeps_the_conversation_full_size_in_the_smallest_window() {
    let mut app = connected_with_status();
    let smallest = PhysicalSize::new(960, 640);

    act(&mut app, WindowAction::Resize(smallest));

    let layout = canvas_of(&app).shown.layout;
    assert_eq!(layout, Layout::reflowed(smallest, RETINA));
    assert!((layout.font_size() - FULL_SIZE_TEXT).abs() < f32::EPSILON);
}

#[test]
fn keeps_the_sign_in_screen_full_size_in_the_smallest_window() {
    let mut app = signing_in();
    let smallest = PhysicalSize::new(960, 640);

    act(&mut app, WindowAction::Resize(smallest));

    let layout = canvas_of(&app).shown.layout;
    assert_eq!(layout, Layout::zoomed(smallest, RETINA));
    assert!((layout.font_size() - FULL_SIZE_TEXT).abs() < f32::EPSILON);
}

#[test]
fn zooms_the_sign_in_screen_out_in_a_window_narrower_than_its_column() {
    let mut app = signing_in();
    let narrow = PhysicalSize::new(800, 640);

    act(&mut app, WindowAction::Resize(narrow));

    let layout = canvas_of(&app).shown.layout;
    assert_eq!(layout, Layout::zoomed(narrow, RETINA));
    assert!(layout.font_size() < FULL_SIZE_TEXT);
}

#[test]
fn reflows_the_prompt_into_a_narrower_window() {
    let mut app = connected_with_status();
    act(&mut app, typed(&"word ".repeat(14)));
    let wide = flow_texts(&app);

    act(&mut app, WindowAction::Resize(PhysicalSize::new(880, 1800)));

    assert_eq!(wide[0], format!("› {}", "word ".repeat(14)));
    assert_eq!(
        flow_texts(&app)[..2],
        [
            format!("› {}w", "word ".repeat(11)),
            "ord word word ".to_owned()
        ]
    );
}

#[test]
fn reads_the_session_again_when_the_window_comes_back() {
    let (mut app, mailbox) = app_and_mailbox();

    assert_eq!(act(&mut app, WindowAction::Focus), Flow::Continue);

    let delivered = mailbox.recv_timeout(PATIENCE).unwrap();
    assert!(matches!(delivered, Delivery::Session(status) if status == FixedSession::default().0));
}

#[test]
fn an_event_asks_for_a_frame_and_the_scene_is_composed_when_painting() {
    let mut app = connected_with_status();
    let before = canvas_of(&app).shown.clone();
    let now = Instant::now();

    app.handle(typed("x"), now);
    let requested = canvas_of(&app).frames_requested;
    let shown_before_painting = canvas_of(&app).shown.clone();
    app.handle(WindowAction::Paint, now);

    assert!(requested > 0);
    assert_eq!(shown_before_painting, before);
    assert!(flow_texts(&app).iter().any(|text| text.contains('x')));
}

#[test]
fn stays_on_the_sign_in_screen_while_nothing_is_connected() {
    let mut app = app_with(RecordingCanvas::default());

    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(None))),
        Instant::now(),
    );

    assert!(matches!(app.screen, Screen::SignIn));
}

#[test]
fn a_late_sign_in_report_keeps_the_conversation() {
    let mut app = connected_with_status();
    act(&mut app, typed("kept"));

    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );

    assert!(flow_texts(&app).iter().any(|text| text.contains("kept")));
}

#[test]
fn neither_composes_nor_paints_while_the_window_is_hidden_and_paints_once_shown() {
    let mut app = connected_with_status();
    let painted = canvas_of(&app).paint_count;

    app.handle(WindowAction::Hide, Instant::now());
    app.handle(WindowAction::Paint, Instant::now());
    let while_hidden = canvas_of(&app).paint_count;
    app.handle(WindowAction::Show, Instant::now());

    assert_eq!(while_hidden, painted);
    assert_eq!(canvas_of(&app).paint_count, painted + 1);
    assert!(!app.window.hidden);
}
