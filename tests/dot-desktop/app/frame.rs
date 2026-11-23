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
    let layout = app.layout();
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
    }
    settled(&mut app);

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
