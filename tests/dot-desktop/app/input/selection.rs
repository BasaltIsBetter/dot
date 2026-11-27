use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;

use crate::app::tests::{TestApp, act, connected_with_status, named};

const SETTLING: Duration = Duration::from_millis(1_500);
use crate::event::WindowAction;

fn keystroke(key: winit::keyboard::Key, super_key: bool) -> crate::event::Keystroke {
    let text = match &key {
        winit::keyboard::Key::Character(character) => Some(character.clone()),
        _ => None,
    };
    crate::event::Keystroke {
        key,
        text,
        modifiers: if super_key {
            winit::keyboard::ModifiersState::SUPER
        } else {
            winit::keyboard::ModifiersState::empty()
        },
    }
}

#[test]
fn counts_up_to_three_clicks_close_in_time_and_place() {
    let (mut app, _receiver) = crate::app::tests::app_and_mailbox();
    let now = Instant::now();

    assert_eq!(app.count_click([10.0, 10.0], now), 1);
    assert_eq!(
        app.count_click([12.0, 13.0], now + Duration::from_millis(200)),
        2
    );
    assert_eq!(
        app.count_click([12.0, 13.0], now + Duration::from_millis(400)),
        3
    );
    assert_eq!(
        app.count_click([12.0, 13.0], now + Duration::from_millis(600)),
        1
    );
    assert_eq!(
        app.count_click([40.0, 13.0], now + Duration::from_millis(700)),
        1
    );
    assert_eq!(
        app.count_click([40.0, 13.0], now + Duration::from_millis(1_300)),
        1
    );
}

fn with_a_reply() -> TestApp {
    let (mut app, receiver) = crate::app::tests::chatting(crate::reply::tests::ScriptedChat {
        pieces: vec![dot_api::ReplyEvent::Text("alpha beta gamma".into())],
        ..crate::reply::tests::ScriptedChat::default()
    });
    act(&mut app, crate::app::tests::typed("question"));
    act(&mut app, named(winit::keyboard::NamedKey::Enter));
    crate::app::tests::until_replied(&mut app, &receiver);
    std::thread::sleep(SETTLING);
    app.handle(WindowAction::Paint, Instant::now());
    app
}

fn spot_of(app: &TestApp, word: &str) -> [f32; 2] {
    let pane = &crate::app::tests::canvas_of(app).shown.panes[0];
    let advance = app.layout(Instant::now()).advance();
    pane.lines
        .iter()
        .find_map(|placed| {
            let text: String = placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            let column = text.find(word)?;
            let left = pane.left + f32::from(placed.line.indent) * advance;
            Some([left + to_columns(column) * advance + 1.0, placed.top + 2.0])
        })
        .unwrap()
}

fn to_columns(bytes: usize) -> f32 {
    f32::from(u16::try_from(bytes).unwrap())
}

fn point(app: &mut TestApp, [x, y]: [f32; 2]) {
    act(
        app,
        WindowAction::Point(PhysicalPosition::new(f64::from(x), f64::from(y))),
    );
}

#[test]
fn a_drag_over_a_reply_selects_its_words_and_escape_clears_them() {
    let mut app = with_a_reply();
    let start = spot_of(&app, "alpha");
    let end = spot_of(&app, "eta");

    point(&mut app, start);
    act(&mut app, WindowAction::Press);
    point(&mut app, end);
    act(&mut app, WindowAction::Release);

    let conversation = &app.sessions.active().conversation;
    assert_eq!(conversation.selected_text().as_deref(), Some("alpha beta"));
    act(&mut app, named(winit::keyboard::NamedKey::Escape));
    assert_eq!(app.sessions.active().conversation.selected_text(), None);
}

#[test]
fn escape_without_a_selection_goes_on_to_the_conversation() {
    let mut app = with_a_reply();

    let used = app.selection_key(
        &keystroke(
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            false,
        ),
        Instant::now(),
    );

    assert!(!used);
}

#[test]
fn command_c_without_a_selection_copies_nothing() {
    let mut app = with_a_reply();

    let used = app.selection_key(
        &keystroke(winit::keyboard::Key::Character("c".into()), true),
        Instant::now(),
    );

    assert!(!used);
}

#[test]
fn hides_the_reticle_over_text_for_the_beam() {
    let mut app = with_a_reply();
    let on_text = spot_of(&app, "alpha");
    let now = Instant::now();
    let layout = app.layout(now);

    point(&mut app, on_text);
    assert_eq!(app.reticle(&layout, now), None);
    point(&mut app, [5.0, 5.0]);
    assert!(app.reticle(&layout, now).is_some());
}

#[test]
fn switching_session_clears_the_selection() {
    let mut app = with_a_reply();
    let start = spot_of(&app, "alpha");
    point(&mut app, start);
    act(&mut app, WindowAction::Press);
    act(&mut app, WindowAction::Press);
    act(&mut app, WindowAction::Release);
    assert!(app.sessions.active().conversation.selected_text().is_some());

    app.open_session();
    app.sessions.switch_to(0);

    assert_eq!(app.sessions.active().conversation.selected_text(), None);
}

#[test]
fn the_pointer_drags_the_selection_only_while_the_button_is_down() {
    let mut app = connected_with_status();

    act(&mut app, WindowAction::Press);
    assert!(app.window.pressing);
    act(&mut app, WindowAction::Release);

    assert!(!app.window.pressing);
}
