use std::time::Duration;

use dot_api::ReplyEvent;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::*;
use crate::app::tests::{
    TestApp, act, canvas_of, chatting, deliver, named, settled, shown_strip, typed,
};
use crate::event::{Keystroke, WindowAction};
use crate::reply::tests::ScriptedChat;
use crate::sessions::Hit;

const PATIENCE: Duration = Duration::from_secs(5);

fn all_texts(app: &TestApp) -> Vec<String> {
    canvas_of(app)
        .shown
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .map(|placed| {
            placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        })
        .collect()
}

fn pixels(count: u32) -> impl Iterator<Item = f32> {
    (0..count).map(|pixel| f32::from(u16::try_from(pixel).unwrap()))
}

fn spot_of(app: &TestApp, wanted: Hit) -> [f32; 2] {
    let layout = app.layout(Instant::now());
    let rail = app
        .rail(&layout, Instant::now())
        .expect("a wide window has room for the rail");
    let size = app.window.size;
    pixels(size.height)
        .flat_map(|y| pixels(size.width).map(move |x| [x, y]))
        .find(|&spot| rail.hit(spot) == Some(wanted))
        .expect("the rail shows what is wanted")
}

fn point_at(app: &mut TestApp, [x, y]: [f32; 2]) {
    act(
        app,
        WindowAction::Point(PhysicalPosition::new(f64::from(x), f64::from(y))),
    );
}

fn click(app: &mut TestApp, wanted: Hit) {
    let spot = spot_of(app, wanted);
    point_at(app, spot);
    act(app, WindowAction::Press);
}

fn command_down() -> WindowAction {
    WindowAction::Key(Keystroke {
        key: Key::Named(NamedKey::ArrowDown),
        text: None,
        modifiers: ModifiersState::SUPER,
    })
}

#[test]
fn shows_each_session_as_a_card_beside_the_conversation() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());

    act(&mut app, WindowAction::Paint);

    let texts = all_texts(&app);
    assert!(texts.contains(&"Dot".to_owned()), "{texts:?}");
    assert!(texts.contains(&"◉".to_owned()), "{texts:?}");
}

#[test]
fn the_new_card_opens_a_session_and_command_down_comes_back() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());
    act(&mut app, typed("kept"));

    click(&mut app, Hit::Open);
    settled(&mut app);

    assert_eq!(app.sessions.count(), "2/2");
    let texts = all_texts(&app);
    assert!(texts.contains(&"session-name 2".to_owned()), "{texts:?}");
    assert!(!texts.iter().any(|text| text.contains("kept")), "{texts:?}");

    act(&mut app, command_down());
    settled(&mut app);

    assert_eq!(app.sessions.count(), "1/2");
    assert!(all_texts(&app).iter().any(|text| text.contains("kept")));
}

#[test]
fn a_reply_that_ends_in_another_session_leaves_it_to_read() {
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bonjour".into())],
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));
    click(&mut app, Hit::Open);

    while app.sessions.unread() == 0 {
        let delivery = receiver.recv_timeout(PATIENCE).unwrap();
        deliver(&mut app, delivery, Instant::now());
        settled(&mut app);
    }

    assert!(all_texts(&app).contains(&"◆".to_owned()));

    act(&mut app, command_down());

    assert_eq!(app.sessions.unread(), 0);
}

#[test]
fn the_cross_of_a_card_closes_its_session() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());
    click(&mut app, Hit::Open);
    let first = spot_of(&app, Hit::Switch(0));
    point_at(&mut app, first);

    click(&mut app, Hit::Close(0));

    assert_eq!(app.sessions.count(), "1/1");
}

#[test]
fn counts_the_sessions_in_the_bar_when_the_window_has_no_room_for_the_rail() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());
    act(&mut app, WindowAction::Rescale(1.0));
    act(&mut app, WindowAction::Resize(PhysicalSize::new(870, 800)));

    act(&mut app, WindowAction::Paint);

    assert!(shown_strip(&app).contains(" 1/1"), "{}", shown_strip(&app));
}

fn asked(app: &mut TestApp) {
    act(app, typed("hello"));
    act(app, named(NamedKey::Enter));
}

fn paced_reply(conversation: &mut crate::conversation::Conversation, start: Instant) {
    let text = ReplyEvent::Text("un deux trois quatre cinq six".into());
    conversation.hear(text, start, &crate::app::tests::IdTranslator);
    assert!(conversation.end_reply(None, Some(6), start).is_none());
}

fn frames(app: &mut TestApp, start: Instant) {
    for frame in 1..=30 {
        app.compose(start + Duration::from_millis(100 * frame));
    }
}

#[test]
fn shows_a_reply_word_by_word_and_wakes_until_it_is_all_shown() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());
    asked(&mut app);
    let start = Instant::now();

    paced_reply(&mut app.sessions.active_mut().conversation, start);

    assert!(app.sessions.active().conversation.is_pacing());
    assert_eq!(
        app.pacing_wake(start),
        Some(start + Duration::from_millis(100))
    );
    frames(&mut app, start);
    assert!(!app.sessions.active().conversation.is_live());
    assert_eq!(app.pacing_wake(start), None);
    assert_eq!(app.sessions.unread(), 0);
}

#[test]
fn leaves_a_session_to_read_once_its_paced_reply_is_all_shown() {
    let (mut app, _receiver) = chatting(ScriptedChat::default());
    asked(&mut app);
    click(&mut app, Hit::Open);
    let start = Instant::now();
    let (_, first) = app.sessions.each_mut().find(|(active, _)| !active).unwrap();
    paced_reply(&mut first.conversation, start);

    app.compose(start);
    let while_shown = app.sessions.unread();
    frames(&mut app, start);

    assert_eq!(while_shown, 0);
    assert_eq!(app.sessions.unread(), 1);
}
