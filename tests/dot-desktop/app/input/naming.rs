use std::time::{Duration, Instant};

use dot_api::{Hue, Toggle};
use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use crate::app::tests::{TestApp, act, connected_with_status, named, settled, shown_strip, typed};
use crate::event::WindowAction;
use crate::sessions::Hit;

fn spot(app: &TestApp, hit: Hit) -> [f32; 2] {
    let now = Instant::now();
    app.rail(&app.layout(now), now)
        .and_then(|rail| rail.spot_of(hit))
        .unwrap()
}

fn click(app: &mut TestApp, [x, y]: [f32; 2]) {
    act(
        app,
        WindowAction::Point(PhysicalPosition::new(f64::from(x), f64::from(y))),
    );
    act(app, WindowAction::Press);
}

fn renaming_the_first() -> TestApp {
    let mut app = connected_with_status();
    settled(&mut app);
    let name = spot(&app, Hit::Name(0));
    click(&mut app, name);
    app
}

#[test]
fn a_click_on_a_name_opens_it_whole_for_typing() {
    let app = renaming_the_first();

    let field = app.renaming.as_ref().unwrap();
    assert_eq!(field.text, "Dot");
    assert!(field.whole);
}

#[test]
fn enter_keeps_the_new_name_in_the_card_and_the_bar() {
    let mut app = renaming_the_first();

    act(&mut app, typed("api"));
    act(&mut app, named(NamedKey::Enter));
    settled(&mut app);

    assert_eq!(app.renaming, None);
    assert_eq!(app.sessions.active().identity.name.as_deref(), Some("api"));
    assert!(shown_strip(&app).contains(" api "), "{}", shown_strip(&app));
}

#[test]
fn escape_keeps_the_old_name_and_reaches_nothing_else() {
    let mut app = renaming_the_first();
    act(&mut app, typed("api"));

    act(&mut app, named(NamedKey::Escape));

    assert_eq!(app.renaming, None);
    assert_eq!(app.sessions.active().identity.name, None);
}

#[test]
fn a_click_elsewhere_keeps_what_was_typed() {
    let mut app = renaming_the_first();
    act(&mut app, typed("api"));

    click(&mut app, [10.0, 10.0]);

    assert_eq!(app.renaming, None);
    assert_eq!(app.sessions.active().identity.name.as_deref(), Some("api"));
}

#[test]
fn a_click_inside_the_field_keeps_typing() {
    let mut app = renaming_the_first();
    act(&mut app, typed("a"));
    let field = spot(&app, Hit::Name(0));

    click(&mut app, field);

    assert_eq!(
        app.renaming.as_ref().map(|field| field.text.as_str()),
        Some("a")
    );
}

#[test]
fn an_empty_name_keeps_the_old_one() {
    let mut app = renaming_the_first();

    act(&mut app, named(NamedKey::Backspace));
    act(&mut app, named(NamedKey::Enter));

    assert_eq!(app.sessions.active().identity.name, None);
}

#[test]
fn moves_the_caret_and_takes_composed_text() {
    let mut app = renaming_the_first();

    act(&mut app, named(NamedKey::ArrowRight));
    act(&mut app, named(NamedKey::ArrowLeft));
    act(&mut app, WindowAction::Commit("é".to_owned()));
    act(&mut app, named(NamedKey::Enter));

    assert_eq!(app.sessions.active().identity.name.as_deref(), Some("Doét"));
}

#[test]
fn ignores_shortcuts_while_renaming() {
    let mut app = renaming_the_first();
    act(&mut app, typed("a"));

    act(
        &mut app,
        WindowAction::Key(crate::event::Keystroke {
            key: winit::keyboard::Key::Character("n".into()),
            text: Some("n".into()),
            modifiers: winit::keyboard::ModifiersState::SUPER,
        }),
    );

    assert_eq!(
        app.renaming.as_ref().map(|field| field.text.as_str()),
        Some("a")
    );
}

#[test]
fn tints_tools_and_states_only_as_the_settings_say() {
    let mut app = connected_with_status();
    app.sessions.paint(Hue::Cyan);

    let colours = app.colours();
    assert_eq!(colours.tool, Some(Hue::Cyan.rgb()));
    assert!(colours.states);
    app.settings.tool_colors = Toggle::Off;
    app.settings.state_colors = Toggle::Off;
    let colours = app.colours();
    assert_eq!(colours.tool, None);
    assert!(!colours.states);
}

#[test]
fn a_new_name_cools_in_a_second_and_a_fifth() {
    let mut app = renaming_the_first();
    act(&mut app, typed("api"));
    act(&mut app, named(NamedKey::Enter));
    let renamed = app.sessions.active().identity.renamed.unwrap();

    assert!(
        app.sessions
            .names_are_cooling(renamed + Duration::from_millis(1_100))
    );
    assert!(
        !app.sessions
            .names_are_cooling(renamed + Duration::from_millis(1_300))
    );
}

#[test]
fn a_new_name_cools_in_the_bar_from_the_session_s_colour() {
    let mut app = renaming_the_first();
    app.sessions.paint(Hue::Cyan);
    act(&mut app, typed("api"));
    act(&mut app, named(NamedKey::Enter));
    let renamed = app.sessions.active().identity.renamed.unwrap();
    let tint_at = |app: &mut TestApp, at: Instant| {
        let mut activity = crate::status_bar::Activity::default();
        app.dress(&mut activity, 0, at);
        activity.name_tint
    };

    assert_eq!(tint_at(&mut app, renamed), Some(Hue::Cyan.rgb()));
    let halfway = tint_at(&mut app, renamed + Duration::from_millis(300));
    assert!(halfway.is_some_and(|tint| tint != Hue::Cyan.rgb() && tint != [u8::MAX; 3]));
    assert_eq!(
        tint_at(&mut app, renamed + Duration::from_millis(1_300)),
        None
    );
}
