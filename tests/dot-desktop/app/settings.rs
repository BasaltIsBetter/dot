use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use dot_api::{
    CaretBlink, Column, Decision, Effort, Hue, Language, Model, PermissionMode, ReadingLine,
    SessionsColumn,
};
use winit::dpi::PhysicalPosition;
use winit::keyboard::{ModifiersState, NamedKey, SmolStr};

use super::*;
use crate::app::tests::{
    IdTranslator, RETINA, RETINA_SIZE, RecordingCanvas, TestApp, act, canvas_of, choices_with,
    connected_with_status, deliver, flow_texts, language_used, named, opener, settled, shown_strip,
    test_keep, typed, until_replied,
};
use crate::editor::tests::MemoryWorkspace;
use crate::event::{Delivery, Wheel, WindowAction};
use crate::reply::tests::{ScriptedChat, connected, wait_until};
use crate::settings::tests::{MemoryPreferences, test_settings};
use crate::sign_in::Report;
use crate::status_bar::FixedSession;
use dot_api::Connection;

fn command(text: &str) -> WindowAction {
    WindowAction::Key(Keystroke {
        key: Key::Character(SmolStr::new(text)),
        text: Some(SmolStr::new(text)),
        modifiers: ModifiersState::SUPER,
    })
}

fn keeping_in(preferences: MemoryPreferences, chat: ScriptedChat) -> (TestApp, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let mut app = DesktopApp::new(
        &IdTranslator,
        connected(),
        opener(chat),
        (choices_with(preferences), test_keep()),
        (sender, Arc::new(MemoryWorkspace::default())),
    );
    app.canvas = Some(RecordingCanvas::default());
    app.window.size = RETINA_SIZE;
    app.window.scale_factor = RETINA;
    app.window.focused = true;
    let now = Instant::now();
    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        now,
    );
    deliver(&mut app, Delivery::Session(FixedSession::default().0), now);
    (app, receiver)
}

fn opened_on(steps_down: usize) -> TestApp {
    let mut app = connected_with_status();
    act(&mut app, command(","));
    for _ in 0..steps_down {
        act(&mut app, named(NamedKey::ArrowDown));
    }
    app
}

fn click(app: &mut TestApp, [x, y]: [f32; 2]) {
    act(
        app,
        WindowAction::Point(PhysicalPosition::new(f64::from(x), f64::from(y))),
    );
    act(app, WindowAction::Press);
}

fn strip_spot(app: &TestApp, wanted: Target) -> [f32; 2] {
    let layout = app.layout(Instant::now());
    let strip = app.strip.as_ref().unwrap();
    let [start, _] = strip
        .targets
        .iter()
        .find(|(target, _)| *target == wanted)
        .unwrap()
        .1;
    [layout.strip_x(start) + 1.0, layout.strip_middle()]
}

fn shown_texts(app: &TestApp) -> Vec<String> {
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

#[test]
fn command_comma_opens_the_settings_over_the_conversation_and_closes_them() {
    let mut app = connected_with_status();

    act(&mut app, command(","));
    settled(&mut app);

    assert!(app.settings_screen.is_open());
    assert!(shown_texts(&app).contains(&Text::SettingModel.id().to_owned()));
    assert!(
        shown_strip(&app).contains(Text::SettingsTitle.id()),
        "{}",
        shown_strip(&app)
    );
    act(&mut app, command(","));
    settled(&mut app);
    assert!(!app.settings_screen.is_open());
    assert!(!shown_texts(&app).contains(&Text::SettingModel.id().to_owned()));
    assert!(shown_strip(&app).contains("Opus 5.5"));
}

#[test]
fn escape_closes_the_settings() {
    let mut app = opened_on(0);

    act(&mut app, named(NamedKey::Escape));

    assert!(!app.settings_screen.is_open());
}

#[test]
fn never_opens_the_settings_before_sign_in() {
    let mut app = crate::app::tests::signing_in();

    act(&mut app, command(","));

    assert!(!app.settings_screen.is_open());
}

#[test]
fn a_new_model_reaches_every_conversation_and_is_kept() {
    let preferences = MemoryPreferences::default();
    let chat = ScriptedChat::default();
    let tunings = Arc::clone(&chat.tunings);
    let (mut app, _receiver) = keeping_in(preferences.clone(), chat);
    act(&mut app, command(","));

    act(&mut app, named(NamedKey::ArrowLeft));

    assert_eq!(app.settings.tuning.model, Model::SONNET_5_5);
    assert_eq!(
        tunings.lock().unwrap().last().map(|tuning| tuning.model),
        Some(Model::SONNET_5_5)
    );
    wait_until("the new model is kept", || {
        preferences
            .saved
            .lock()
            .unwrap()
            .last()
            .map(|kept| kept.tuning.model)
            == Some(Model::SONNET_5_5)
    });
}

#[test]
fn a_new_effort_is_kept_but_nothing_is_saved_without_a_change() {
    let preferences = MemoryPreferences::default();
    let (mut app, _receiver) = keeping_in(preferences.clone(), ScriptedChat::default());
    act(&mut app, command(","));
    act(&mut app, named(NamedKey::ArrowDown));
    act(&mut app, named(NamedKey::ArrowDown));
    act(&mut app, named(NamedKey::ArrowUp));

    act(&mut app, named(NamedKey::ArrowRight));
    act(&mut app, command(","));
    app.keeper.stop();

    assert_eq!(app.settings.tuning.effort, Effort::ExtraHigh);
    assert_eq!(preferences.saved.lock().unwrap().len(), 1);
}

#[test]
fn the_status_bar_glows_on_a_new_model_once_the_settings_close() {
    let mut app = opened_on(0);
    act(&mut app, named(NamedKey::ArrowLeft));
    let now = Instant::now();
    assert_eq!(app.tuned_heats(now), [0, 0]);

    app.close_settings(now);

    assert_eq!(app.tuned_heats(now), [u8::MAX, 0]);
    assert!(app.settings_are_moving(now + Duration::from_millis(1_100)));
    assert_eq!(app.tuned_heats(now + Duration::from_millis(1_200)), [0, 0]);
}

#[test]
fn only_the_effort_glows_when_only_the_effort_changed() {
    let mut app = opened_on(1);
    act(&mut app, named(NamedKey::ArrowLeft));
    let now = Instant::now();

    app.close_settings(now);

    assert_eq!(app.tuned_heats(now), [0, u8::MAX]);
}

#[test]
fn the_status_bar_stays_quiet_when_the_model_did_not_change() {
    let mut app = opened_on(5);
    act(&mut app, named(NamedKey::ArrowRight));
    let now = Instant::now();

    app.close_settings(now);

    assert_eq!(app.tuned_heats(now), [0, 0]);
}

#[test]
fn a_change_of_permissions_applies_to_the_next_reply() {
    let chat = ScriptedChat {
        asks: vec![dot_api::Permission {
            tool_id: "t1".to_owned(),
            action: dot_api::Action::Run {
                command: "ls".to_owned(),
            },
        }],
        ..ScriptedChat::default()
    };
    let decisions = Arc::clone(&chat.decisions);
    let (mut app, receiver) = crate::app::tests::chatting(chat);
    act(&mut app, command(","));
    for _ in 0..3 {
        act(&mut app, named(NamedKey::ArrowDown));
    }
    act(&mut app, named(NamedKey::ArrowLeft));
    act(&mut app, command(","));
    assert_eq!(app.settings.permissions, PermissionMode::Bypass);

    act(&mut app, typed("liste"));
    act(&mut app, named(NamedKey::Enter));
    until_replied(&mut app, &receiver);

    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
}

#[test]
fn a_new_language_is_used_at_once() {
    let mut app = opened_on(8);

    act(&mut app, named(NamedKey::ArrowRight));

    assert_eq!(app.settings.language, Language::English);
    assert_eq!(language_used(), Some(Language::English));
}

#[test]
fn the_reading_line_glides_to_a_third_of_the_window() {
    let mut app = opened_on(11);
    let now = Instant::now();
    assert!((app.layout(now).reading_line() - 900.0).abs() < 0.5);

    act(&mut app, named(NamedKey::ArrowRight));

    assert_eq!(app.settings.reading_line, ReadingLine::Third);
    let later = Instant::now() + Duration::from_secs(1);
    assert!((app.layout(later).reading_line() - 600.0).abs() < 0.5);
    assert!(app.settings_are_moving(Instant::now() + Duration::from_millis(100)));
}

#[test]
fn the_column_and_the_sessions_column_follow_the_settings() {
    let mut app = connected_with_status();
    let now = Instant::now();
    assert_eq!(app.fit(now).widest, 96);
    assert!(app.fit(now).rail);

    app.settings.column = Column::Narrow;
    app.settings.sessions_column = SessionsColumn::Never;

    assert_eq!(app.fit(now).widest, 64);
    assert!(!app.fit(now).rail);
}

#[test]
fn draws_live_ink_in_the_colour_of_the_session_shown() {
    let mut app = connected_with_status();
    assert_eq!(app.palette().live, Hue::Violet.rgb());

    app.sessions.paint(Hue::Green);

    assert_eq!(app.palette().live, Hue::Green.rgb());
    settled(&mut app);
    assert_eq!(canvas_of(&app).shown.palette.live, Hue::Green.rgb());
}

#[test]
fn a_new_colour_reaches_only_the_session_shown_and_is_kept() {
    let preferences = MemoryPreferences::default();
    let (mut app, _receiver) = keeping_in(preferences.clone(), ScriptedChat::default());
    let first = app.sessions.hue();
    app.open_session();
    app.sessions.paint(Hue::Cyan);
    act(&mut app, command(","));
    let shown = app.settings.color;
    for _ in 0..5 {
        act(&mut app, named(NamedKey::ArrowDown));
    }

    act(&mut app, named(NamedKey::ArrowLeft));

    assert_eq!(shown, Hue::Cyan);
    assert_eq!(app.settings.color, Hue::Green);
    assert_eq!(app.palette().live, Hue::Green.rgb());
    assert_eq!(app.colours().tool, Some(Hue::Green.rgb()));
    app.sessions.switch_to(0);
    assert_eq!(first, Hue::Violet);
    assert_eq!(app.palette().live, first.rgb());
    assert_eq!(app.colours().tool, Some(first.rgb()));
    wait_until("the new colour is kept", || {
        preferences
            .saved
            .lock()
            .unwrap()
            .last()
            .map(|kept| kept.color)
            == Some(Hue::Green)
    });
}

#[test]
fn a_steady_caret_never_wakes_the_window_to_blink() {
    let mut app = connected_with_status();
    settled(&mut app);
    let blinking = app.wake_at;

    app.settings.caret = CaretBlink::Steady;
    let now = Instant::now() + Duration::from_secs(2);
    app.handle(WindowAction::Paint, now);

    let half_a_blink = now + Duration::from_millis(600);
    assert!(blinking.is_some());
    assert!(app.wake_at.is_none_or(|wake| wake > half_a_blink));
}

#[test]
fn clicking_the_model_or_the_effort_in_the_status_bar_opens_its_setting() {
    for (target, setting) in [
        (Target::Model, Setting::Model),
        (Target::Effort, Setting::Effort),
    ] {
        let mut app = connected_with_status();
        settled(&mut app);
        let spot = strip_spot(&app, target);

        click(&mut app, spot);

        assert!(app.settings_screen.is_open(), "{target:?}");
        assert_eq!(app.settings_screen.focus(), setting);
    }
}

#[test]
fn clicking_the_gear_opens_then_closes_the_settings() {
    let mut app = connected_with_status();
    settled(&mut app);

    let gear = strip_spot(&app, Target::Settings);
    click(&mut app, gear);
    assert!(app.settings_screen.is_open());
    settled(&mut app);
    let gear = strip_spot(&app, Target::Settings);
    click(&mut app, gear);

    assert!(!app.settings_screen.is_open());
}

#[test]
fn a_click_in_the_settings_moves_the_focus_there() {
    let mut app = opened_on(0);
    settled(&mut app);
    let layout = app.layout(Instant::now());
    let frame = settings_frame(&layout);
    let below_the_hint = layout.scaled(30.0 + 26.0);

    click(
        &mut app,
        [frame.left + 2.0, frame.band_top + below_the_hint + 2.0],
    );

    assert!(app.settings_screen.is_open());
    assert_eq!(app.settings_screen.focus(), Setting::Effort);
}

#[test]
fn command_and_the_wheel_open_the_settings_and_the_wheel_then_moves_through_them() {
    let mut app = connected_with_status();
    act(&mut app, WindowAction::Modifiers(ModifiersState::SUPER));

    act(&mut app, WindowAction::Scroll(Wheel::Lines(-1.0)));
    assert!(app.settings_screen.is_open());
    assert_eq!(app.settings_screen.focus(), Setting::Model);

    act(&mut app, WindowAction::Modifiers(ModifiersState::empty()));
    app.settings_wheel(40.0, Instant::now() + Duration::from_secs(1));
    assert_eq!(app.settings_screen.focus(), Setting::Effort);
}

#[test]
fn what_is_typed_searches_the_settings_and_leaves_the_prompter_alone() {
    let mut app = opened_on(0);

    act(&mut app, WindowAction::Commit("eff".to_owned()));
    act(&mut app, WindowAction::Compose("x".to_owned()));
    assert_eq!(app.searching(Instant::now()).query, "eff");
    act(&mut app, command(","));
    settled(&mut app);

    let typed_in_the_prompter = flow_texts(&app)
        .into_iter()
        .filter(|text| !text.starts_with("suggestion-"))
        .any(|text| text.contains("eff") || text.contains('x'));
    assert!(!typed_in_the_prompter, "{:?}", flow_texts(&app));
}

#[test]
fn the_status_bar_shows_the_search_while_the_settings_are_open() {
    let mut app = opened_on(0);

    act(&mut app, typed("e"));
    act(&mut app, typed("f"));
    let searching = app.searching(Instant::now());

    assert_eq!(searching.query, "ef");
    assert_eq!(searching.matching, 1);
    assert_eq!(app.settings_screen.focus(), Setting::Effort);
}

#[test]
fn the_status_bar_glows_on_saved_after_a_change() {
    let mut app = opened_on(0);
    act(&mut app, named(NamedKey::ArrowLeft));

    let now = Instant::now();
    assert!(app.searching(now).saved_heat > 0);
    assert_eq!(
        app.searching(now + Duration::from_millis(450)).saved_heat,
        0
    );
}

#[test]
fn tells_the_user_when_the_settings_could_not_be_kept() {
    let mut app = connected_with_status();

    deliver(
        &mut app,
        Delivery::Settings(SettingsNews::Unsaved(dot_api::PreferencesError(
            "disk full".to_owned(),
        ))),
        Instant::now(),
    );
    deliver(
        &mut app,
        Delivery::Settings(SettingsNews::Saved),
        Instant::now(),
    );

    let notices = flow_texts(&app)
        .iter()
        .filter(|text| text.as_str() == Text::SettingsUnsaved.id())
        .count();
    assert_eq!(notices, 1);
}

#[test]
fn opens_the_settings_on_the_reading_line_glide_settled() {
    let glide = reading_glide(&Settings {
        reading_line: ReadingLine::Third,
        ..test_settings()
    });

    assert!((glide.position(Instant::now()) - 1.0 / 3.0).abs() < f32::EPSILON);
    assert!(!glide.is_moving(Instant::now()));
}

#[test]
fn hands_the_pointer_to_the_settings_where_it_is() {
    let mut app = connected_with_status();
    act(
        &mut app,
        WindowAction::Point(PhysicalPosition::new(120.0, 340.0)),
    );
    let defaults = test_settings();

    assert_eq!(
        app.settings_look((&defaults, &Actions::default())).hover,
        Some([120.0, 340.0])
    );
    act(&mut app, WindowAction::Leave);
    assert_eq!(
        app.settings_look((&defaults, &Actions::default())).hover,
        None
    );
}

#[test]
fn the_wheel_alone_moves_through_open_settings() {
    let mut app = opened_on(0);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(-1.0)));

    assert_eq!(app.settings_screen.focus(), Setting::Effort);
}

#[test]
fn saved_cools_from_the_live_colour_in_four_hundred_and_fifty_milliseconds() {
    let mut app = opened_on(0);
    act(&mut app, named(NamedKey::ArrowLeft));
    let changed = app.settings_screen.changed_at().unwrap();

    let halfway = app
        .searching(changed + Duration::from_millis(225))
        .saved_heat;

    assert!(halfway > 0 && halfway < u8::MAX, "{halfway}");
}

#[test]
fn a_new_model_cools_in_the_status_bar_in_a_second_and_a_fifth() {
    let mut app = opened_on(0);
    act(&mut app, named(NamedKey::ArrowLeft));
    let now = Instant::now();
    app.close_settings(now);

    let [halfway, effort] = app.tuned_heats(now + Duration::from_millis(600));
    assert_eq!(effort, 0);

    assert!(halfway > 0 && halfway < u8::MAX, "{halfway}");
}

#[test]
fn keeps_painting_while_any_one_part_of_the_settings_moves() {
    let mut app = connected_with_status();
    let start = Instant::now() + Duration::from_secs(10);
    assert!(!app.settings_are_moving(start));

    app.open_settings(None, false, start);
    assert!(app.settings_are_moving(start + Duration::from_millis(100)));
    let open = start + Duration::from_secs(1);
    assert!(!app.settings_are_moving(open));

    app.reading.toward(1.0 / 3.0, open);
    assert!(app.settings_are_moving(open + Duration::from_millis(100)));
    let settled = open + Duration::from_secs(1);
    assert!(!app.settings_are_moving(settled));

    app.effort_tuned_at = Some(settled);
    assert!(app.settings_are_moving(settled + Duration::from_millis(1_100)));
    assert!(!app.settings_are_moving(settled + Duration::from_millis(1_300)));
}

#[test]
fn fades_the_conversation_out_as_the_settings_fade_in() {
    let start = Instant::now() + Duration::from_secs(10);
    let halfway = start + Duration::from_millis(100);
    let mut plain = connected_with_status();
    plain.handle(WindowAction::Paint, halfway);
    let mut app = connected_with_status();
    app.open_settings(None, false, start);

    app.handle(WindowAction::Paint, halfway);

    let before = canvas_of(&plain).shown.panes[0].lines[0].line.alpha;
    let shown = app.settings_screen.shown(halfway);
    let faded = canvas_of(&app).shown.panes[0].lines[0].line.alpha;
    let expected = f64::from(before) * (1.0 - shown);
    assert!(shown > 0.1 && shown < 0.9, "{shown}");
    assert!(
        (f64::from(faded) - expected).abs() <= 2.0,
        "{faded} {expected}"
    );
}

#[test]
fn draws_everything_larger_as_soon_as_the_size_grows() {
    let mut app = opened_on(9);
    let now = Instant::now();
    assert_eq!(app.settings_screen.focus(), Setting::Size);
    let before = app.layout(now);

    act(&mut app, named(NamedKey::ArrowRight));

    assert_eq!(app.settings.size, dot_api::InterfaceSize::Medium);
    let after = app.layout(now);
    assert!(after.scale() > before.scale());
    assert!(after.font_size() > before.font_size());
    assert!(after.columns() <= before.columns());
}

#[test]
fn hands_every_conversation_the_compaction_chosen_and_keeps_it() {
    let preferences = MemoryPreferences::default();
    let chat = ScriptedChat::default();
    let tunings = Arc::clone(&chat.tunings);
    let (mut app, _receiver) = keeping_in(preferences.clone(), chat);
    act(&mut app, command(","));
    for _ in 0..17 {
        act(&mut app, named(NamedKey::ArrowDown));
    }
    assert!(matches!(app.settings_screen.focus(), Setting::Output(_)));

    act(&mut app, named(NamedKey::ArrowLeft));

    assert_eq!(
        app.settings.tuning.compaction.contents_cut,
        dot_api::CutLevel::Late
    );
    assert_eq!(
        tunings
            .lock()
            .unwrap()
            .last()
            .map(|tuning| tuning.compaction.contents_cut),
        Some(dot_api::CutLevel::Late)
    );
    wait_until("the new cut is kept", || {
        preferences
            .saved
            .lock()
            .unwrap()
            .last()
            .is_some_and(|saved| saved.tuning.compaction.contents_cut == dot_api::CutLevel::Late)
    });
}
