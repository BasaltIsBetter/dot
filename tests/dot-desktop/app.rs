use std::borrow::Cow;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Connection, Text};

use super::*;
use crate::canvas::Line;
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;

const RETINA_SIZE: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
const RETINA: f64 = 2.0;
const API_KEY_ROW: usize = 4;
const PATIENCE: Duration = Duration::from_secs(5);

struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
}

#[derive(Default)]
struct RecordingCanvas {
    resized_to: Option<PhysicalSize<u32>>,
    shown: Vec<Line>,
    shown_strip: Option<Strip>,
    paint_count: u32,
    fails_to_paint: bool,
}

impl Canvas for RecordingCanvas {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.resized_to = Some(size);
    }

    fn show(&mut self, lines: Vec<Line>, strip: Option<Strip>) {
        self.shown = lines;
        self.shown_strip = strip;
    }

    fn paint(&mut self) -> Result<(), DesktopError> {
        self.paint_count += 1;
        if self.fails_to_paint {
            return Err(DesktopError::SurfaceLost);
        }
        Ok(())
    }
}

type TestApp = DesktopApp<
    'static,
    IdTranslator,
    FakeConnector,
    FixedSession,
    Sender<Delivery>,
    RecordingCanvas,
>;

fn app_and_mailbox() -> (TestApp, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let mut app = DesktopApp::new(
        &IdTranslator,
        FakeConnector::default(),
        FixedSession::default(),
        sender,
    );
    app.canvas = Some(RecordingCanvas::default());
    app.window.size = RETINA_SIZE;
    app.window.scale_factor = RETINA;
    (app, receiver)
}

fn app_with(canvas: RecordingCanvas) -> TestApp {
    let (mut app, _receiver) = app_and_mailbox();
    app.canvas = Some(canvas);
    app
}

fn connected_with_status() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    app.deliver(Delivery::SignIn(Report::Checked(Ok(Some(
        Connection::ApiKey,
    )))));
    app.deliver(Delivery::Session(FixedSession::default().0));
    app
}

fn shown_strip(app: &TestApp) -> String {
    canvas_of(app)
        .shown_strip
        .as_ref()
        .unwrap()
        .text
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn signing_in() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    app.sign_in.report(Report::Checked(Ok(None)));
    app
}

fn canvas_of(app: &TestApp) -> &RecordingCanvas {
    app.canvas.as_ref().unwrap()
}

fn shown_row(app: &TestApp, row: usize) -> String {
    canvas_of(app).shown[row]
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn over_row(row: usize) -> PhysicalPosition<f64> {
    let layout = Layout::new(RETINA_SIZE, RETINA);
    PhysicalPosition::new(
        f64::from(layout.x(0)) + 1.0,
        f64::from(layout.line_top(row)) + 1.0,
    )
}

#[test]
fn closing_the_window_quits_without_error() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(app.handle(WindowAction::Close), Flow::Exit);
    assert!(app.into_result().is_ok());
}

#[test]
fn resizing_the_window_resizes_the_canvas() {
    let mut app = app_with(RecordingCanvas::default());
    let smaller = PhysicalSize::new(1800, 1200);

    assert_eq!(app.handle(WindowAction::Resize(smaller)), Flow::Continue);
    assert_eq!(canvas_of(&app).resized_to, Some(smaller));
    assert_eq!(app.window.size, smaller);
}

#[test]
fn a_redraw_request_paints_the_canvas_once() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(app.handle(WindowAction::Paint), Flow::Continue);
    assert_eq!(canvas_of(&app).paint_count, 1);
}

#[test]
fn a_failed_paint_stops_dot_with_its_error() {
    let mut app = app_with(RecordingCanvas {
        fails_to_paint: true,
        ..RecordingCanvas::default()
    });

    assert_eq!(app.handle(WindowAction::Paint), Flow::Exit);
    assert!(matches!(app.into_result(), Err(DesktopError::SurfaceLost)));
}

#[test]
fn other_events_leave_the_canvas_untouched() {
    let mut app = app_with(RecordingCanvas::default());

    assert_eq!(app.handle(WindowAction::Ignore), Flow::Continue);
    assert_eq!(canvas_of(&app).paint_count, 0);
    assert_eq!(canvas_of(&app).shown, Vec::<Line>::new());
}

#[test]
fn events_before_the_window_opens_change_nothing() {
    let (sender, _receiver) = mpsc::channel();
    let mut app: TestApp = DesktopApp::new(
        &IdTranslator,
        FakeConnector::default(),
        FixedSession::default(),
        sender,
    );

    assert_eq!(app.handle(WindowAction::Paint), Flow::Continue);
    assert_eq!(
        app.handle(WindowAction::Key(KeyIntent::Input(Input::Next))),
        Flow::Continue
    );
    assert!(app.into_result().is_ok());
}

#[test]
fn remembers_the_modifiers_and_the_scale() {
    let mut app = app_with(RecordingCanvas::default());

    app.handle(WindowAction::Modifiers(ModifiersState::SUPER));
    app.handle(WindowAction::Rescale(1.0));

    assert_eq!(app.window.modifiers, ModifiersState::SUPER);
    assert!((app.window.scale_factor - 1.0).abs() < f64::EPSILON);
}

#[test]
fn a_key_moves_the_selection_on_screen() {
    let mut app = signing_in();

    app.handle(WindowAction::Key(KeyIntent::Input(Input::Next)));

    assert!(shown_row(&app, API_KEY_ROW).starts_with("› sign-in-api-key"));
}

#[test]
fn hovering_a_way_selects_it() {
    let mut app = signing_in();

    app.handle(WindowAction::Point(over_row(API_KEY_ROW)));

    assert!(shown_row(&app, API_KEY_ROW).starts_with("› "));
}

#[test]
fn clicking_a_way_chooses_it() {
    let mut app = signing_in();
    app.handle(WindowAction::Point(over_row(API_KEY_ROW)));

    app.handle(WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-key-prompt");
}

#[test]
fn clicking_outside_the_ways_does_nothing() {
    let mut app = signing_in();
    app.handle(WindowAction::Point(over_row(0)));

    app.handle(WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-intro");
}

#[test]
fn a_connected_dot_shows_nothing() {
    let mut app = app_with(RecordingCanvas::default());
    app.sign_in
        .report(Report::Checked(Ok(Some(Connection::ApiKey))));

    app.handle(WindowAction::Key(KeyIntent::Input(Input::Next)));

    assert_eq!(canvas_of(&app).shown, Vec::<Line>::new());
    assert_eq!(canvas_of(&app).shown_strip, None);
}

#[test]
fn shows_the_status_bar_once_connected_and_the_session_read() {
    let app = connected_with_status();

    let strip = shown_strip(&app);
    assert!(strip.trim_start().starts_with("Opus 5.5"), "{strip}");
    assert!(strip.ends_with("Dot  feat/mac-app"), "{strip}");
    assert_eq!(canvas_of(&app).shown, Vec::<Line>::new());
}

#[test]
fn hides_the_status_bar_while_signing_in() {
    let mut app = signing_in();

    app.deliver(Delivery::Session(FixedSession::default().0));

    assert_eq!(canvas_of(&app).shown_strip, None);
    assert_ne!(canvas_of(&app).shown, Vec::<Line>::new());
}

#[test]
fn fits_the_status_bar_again_when_the_window_resizes() {
    let mut app = connected_with_status();
    let wide = shown_strip(&app);

    app.handle(WindowAction::Resize(PhysicalSize::new(960, 640)));

    let narrow = shown_strip(&app);
    assert!(narrow.chars().count() < wide.chars().count());
    assert!(narrow.trim_start().starts_with("Opus 5.5"));
}

#[test]
fn reads_the_session_again_when_the_window_comes_back() {
    let (mut app, mailbox) = app_and_mailbox();

    assert_eq!(app.handle(WindowAction::Focus), Flow::Continue);

    let delivered = mailbox.recv_timeout(PATIENCE).unwrap();
    assert!(matches!(delivered, Delivery::Session(status) if status == FixedSession::default().0));
}
