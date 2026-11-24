use std::sync::{Arc, Mutex};

use dot_api::{
    CaretBlink, Column, Effort, Language, LiveColor, Model, PermissionMode, Preferences,
    PreferencesError, ReadingLine, SessionsColumn, Summaries, Text, Toggle, Tuning,
};
use winit::keyboard::{ModifiersState, SmolStr};

use super::*;
use crate::app::tests::IdTranslator;

pub(crate) const REFUSED: &str = "the disk is full";

#[derive(Clone, Default)]
pub(crate) struct MemoryPreferences {
    pub(crate) saved: Arc<Mutex<Vec<Settings>>>,
    pub(crate) refuses: bool,
}

impl Preferences for MemoryPreferences {
    fn load(&self, defaults: &Settings) -> Settings {
        self.saved
            .lock()
            .unwrap()
            .last()
            .copied()
            .unwrap_or(*defaults)
    }

    fn save(&self, settings: &Settings, _defaults: &Settings) -> Result<(), PreferencesError> {
        if self.refuses {
            return Err(PreferencesError(REFUSED.to_owned()));
        }
        self.saved.lock().unwrap().push(*settings);
        Ok(())
    }
}

pub(crate) const fn test_settings() -> Settings {
    Settings {
        tuning: Tuning {
            model: Model::OPUS_5_5,
            effort: Effort::High,
            summaries: Summaries::WhenTheyPay,
        },
        permissions: PermissionMode::Ask,
        live_color: LiveColor::OfTheSession,
        state_colors: Toggle::On,
        tool_colors: Toggle::On,
        session_colors: Toggle::On,
        language: Language::French,
        column: Column::Wide,
        reading_line: ReadingLine::Middle,
        caret: CaretBlink::Blinking,
        sessions_column: SessionsColumn::Auto,
    }
}

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + std::time::Duration::from_millis(milliseconds)
}

fn opened(now: Instant) -> SettingsScreen {
    let mut screen = SettingsScreen::new();
    screen.open(None, false, now);
    screen
}

fn press(
    screen: &mut SettingsScreen,
    gesture: Gesture,
    settings: &mut Settings,
    now: Instant,
) -> Outcome {
    screen.gesture(gesture, (settings, &test_settings()), &IdTranslator, now)
}

fn stroke(key: Key, text: Option<&str>, modifiers: ModifiersState) -> Keystroke {
    Keystroke {
        key,
        text: text.map(SmolStr::new),
        modifiers,
    }
}

#[test]
fn opens_and_closes_once() {
    let now = Instant::now();
    let mut screen = SettingsScreen::new();
    assert!(!screen.is_open());
    assert_eq!(screen.close(now), Outcome::Nothing);

    screen.open(None, false, now);
    screen.open(Some(Setting::Column), false, after(now, 10));

    assert!(screen.is_open());
    assert_eq!(screen.reel.focus(), Setting::Model);
    assert_eq!(screen.close(now), Outcome::Closed(Vec::new()));
    assert!(!screen.is_open());
}

#[test]
fn opens_on_the_setting_asked_for() {
    let mut screen = SettingsScreen::new();

    screen.open(Some(Setting::Effort), false, Instant::now());

    assert_eq!(screen.reel.focus(), Setting::Effort);
}

#[test]
fn changes_the_setting_in_focus_with_the_side_arrows() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();

    let left = press(&mut screen, Gesture::Left, &mut settings, now);

    assert_eq!(left, Outcome::Changed(Setting::Model));
    assert_eq!(settings.tuning.model, Model::SONNET_5_5);
    assert_eq!(screen.changed_at(), Some(now));
    let right = press(&mut screen, Gesture::Right, &mut settings, now);
    assert_eq!(right, Outcome::Changed(Setting::Model));
    assert_eq!(settings, test_settings());
}

#[test]
fn stops_at_the_first_and_the_last_option() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    Setting::Model.choose(0, &mut settings);

    assert_eq!(
        press(&mut screen, Gesture::Left, &mut settings, now),
        Outcome::Nothing
    );
    Setting::Model.choose(3, &mut settings);
    assert_eq!(
        press(&mut screen, Gesture::Right, &mut settings, now),
        Outcome::Nothing
    );
    assert_eq!(settings.tuning.model, Model::FABLE_5_2);
    assert_eq!(screen.changed_at(), None);
}

#[test]
fn turns_to_the_next_option_with_enter_and_back_to_the_first_after_the_last() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();

    press(&mut screen, Gesture::Enter, &mut settings, now);
    assert_eq!(settings.tuning.model, Model::FABLE_5_2);
    press(&mut screen, Gesture::Enter, &mut settings, now);
    assert_eq!(settings.tuning.model, Model::HAIKU_5_5);
}

#[test]
fn moves_the_focus_with_the_up_and_down_arrows() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();

    assert_eq!(
        press(&mut screen, Gesture::Down, &mut settings, now),
        Outcome::Nothing
    );
    assert_eq!(screen.reel.focus(), Setting::Effort);
    press(&mut screen, Gesture::Up, &mut settings, now);
    assert_eq!(screen.reel.focus(), Setting::Model);
    assert_eq!(settings, test_settings());
}

#[test]
fn erase_brings_the_setting_back_to_its_default() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    Setting::Model.choose(0, &mut settings);

    let erased = press(&mut screen, Gesture::Erase, &mut settings, now);

    assert_eq!(erased, Outcome::Changed(Setting::Model));
    assert_eq!(settings, test_settings());
    assert_eq!(
        press(&mut screen, Gesture::Erase, &mut settings, now),
        Outcome::Nothing
    );
}

#[test]
fn erase_takes_back_a_letter_of_the_search_first() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    Setting::Model.choose(0, &mut settings);
    press(
        &mut screen,
        Gesture::Type("mo".to_owned()),
        &mut settings,
        now,
    );

    assert_eq!(
        press(&mut screen, Gesture::Erase, &mut settings, now),
        Outcome::Nothing
    );
    assert_eq!(screen.query(), "m");
    assert_eq!(settings.tuning.model, Model::HAIKU_5_5);
}

#[test]
fn searches_what_is_typed_but_never_a_space_or_a_control() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();

    press(
        &mut screen,
        Gesture::Type("ef".to_owned()),
        &mut settings,
        now,
    );
    press(
        &mut screen,
        Gesture::Type(" ".to_owned()),
        &mut settings,
        now,
    );
    press(
        &mut screen,
        Gesture::Type("\t".to_owned()),
        &mut settings,
        now,
    );

    assert_eq!(screen.query(), "ef");
    assert_eq!(screen.reel.focus(), Setting::Effort);
    assert_eq!(screen.matching(&IdTranslator), 1);
}

#[test]
fn escape_clears_the_search_then_closes() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    press(
        &mut screen,
        Gesture::Type("ef".to_owned()),
        &mut settings,
        now,
    );
    press(&mut screen, Gesture::Right, &mut settings, now);

    assert_eq!(
        press(&mut screen, Gesture::Escape, &mut settings, now),
        Outcome::Nothing
    );
    assert_eq!(screen.query(), "");
    assert!(screen.is_open());
    assert_eq!(
        press(&mut screen, Gesture::Escape, &mut settings, now),
        Outcome::Closed(vec![Setting::Effort])
    );
}

#[test]
fn fades_in_and_out_in_three_hundred_milliseconds() {
    let now = Instant::now();
    let mut screen = SettingsScreen::new();
    assert!((screen.shown(now) - 0.0).abs() < f64::EPSILON);
    assert!(!screen.is_moving(now));

    screen.open(None, false, now);
    assert!(screen.shown(after(now, 100)) > 0.0);
    assert!(screen.shown(after(now, 100)) < 1.0);
    assert!(screen.is_moving(after(now, 100)));
    assert!((screen.shown(after(now, 400)) - 1.0).abs() < f64::EPSILON);
    assert!(!screen.is_moving(after(now, 400)));

    screen.close(after(now, 400));
    assert!(screen.shown(after(now, 500)) > 0.0);
    assert!(screen.shown(after(now, 500)) < 1.0);
    assert!((screen.shown(after(now, 800)) - 0.0).abs() < f64::EPSILON);
}

#[test]
fn keeps_moving_while_a_changed_value_cools() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    let later = after(now, 400);

    press(&mut screen, Gesture::Right, &mut settings, later);

    assert!(screen.is_moving(after(now, 1_500)));
    assert!(!screen.is_moving(after(now, 1_700)));
}

#[test]
fn reads_the_arrows_enter_escape_and_erase() {
    let none = ModifiersState::empty();
    let read = |key| gesture(&stroke(Key::Named(key), None, none));

    assert_eq!(read(NamedKey::ArrowUp), Some(Gesture::Up));
    assert_eq!(read(NamedKey::ArrowDown), Some(Gesture::Down));
    assert_eq!(read(NamedKey::ArrowLeft), Some(Gesture::Left));
    assert_eq!(read(NamedKey::ArrowRight), Some(Gesture::Right));
    assert_eq!(read(NamedKey::Enter), Some(Gesture::Enter));
    assert_eq!(read(NamedKey::Escape), Some(Gesture::Escape));
    assert_eq!(read(NamedKey::Backspace), Some(Gesture::Erase));
    assert_eq!(read(NamedKey::Shift), None);
}

#[test]
fn reads_a_typed_character_as_search() {
    let typed = stroke(
        Key::Character(SmolStr::new("é")),
        Some("é"),
        ModifiersState::empty(),
    );

    assert_eq!(gesture(&typed), Some(Gesture::Type("é".to_owned())));
}

#[test]
fn leaves_shortcuts_to_the_window() {
    for modifiers in [ModifiersState::SUPER, ModifiersState::CONTROL] {
        let shortcut = stroke(Key::Named(NamedKey::ArrowUp), None, modifiers);
        let letter = stroke(Key::Character(SmolStr::new("a")), Some("a"), modifiers);

        assert_eq!(gesture(&shortcut), None);
        assert_eq!(gesture(&letter), None);
    }
    let shifted = stroke(Key::Named(NamedKey::ArrowUp), None, ModifiersState::SHIFT);
    assert_eq!(gesture(&shifted), Some(Gesture::Up));
}

fn spot_of(screen: &SettingsScreen, settings: &Settings, wanted: Hit) -> [f32; 2] {
    let layout = view::tests::layout();
    let frame = view::tests::frame(&layout);
    let defaults = test_settings();
    let look = view::tests::look(settings, &defaults);
    let drawn = view::entries(&screen.reel, &look, (&layout, &frame), Instant::now());
    let [left, top, width, height] = drawn.hits.iter().find(|(_, hit)| *hit == wanted).unwrap().0;
    [left + width / 2.0, top + height / 2.0]
}

fn clicked(screen: &mut SettingsScreen, settings: &mut Settings, at: [f32; 2]) -> Outcome {
    let layout = view::tests::layout();
    let frame = view::tests::frame(&layout);
    let defaults = test_settings();
    let shown = *settings;
    let look = view::tests::look(&shown, &defaults);
    screen.press(at, (&look, settings), (&layout, &frame), Instant::now())
}

#[test]
fn a_click_on_an_option_of_the_setting_in_focus_chooses_it() {
    let mut screen = opened(Instant::now());
    let mut settings = test_settings();
    let haiku = spot_of(&screen, &settings, Hit::Choose(Setting::Model, 0));

    let outcome = clicked(&mut screen, &mut settings, haiku);

    assert_eq!(outcome, Outcome::Changed(Setting::Model));
    assert_eq!(settings.tuning.model, Model::HAIKU_5_5);
}

#[test]
fn a_click_on_another_setting_brings_it_into_focus_without_changing_it() {
    let mut screen = opened(Instant::now());
    let mut settings = test_settings();
    let effort = spot_of(&screen, &settings, Hit::Choose(Setting::Effort, 2));

    let outcome = clicked(&mut screen, &mut settings, effort);

    assert_eq!(outcome, Outcome::Nothing);
    assert_eq!(screen.focus(), Setting::Effort);
    assert_eq!(settings, test_settings());
}

#[test]
fn a_click_beside_the_settings_does_nothing() {
    let mut screen = opened(Instant::now());
    let mut settings = test_settings();

    let outcome = clicked(&mut screen, &mut settings, [1.0, 1.0]);

    assert_eq!(outcome, Outcome::Nothing);
    assert_eq!(screen.focus(), Setting::Model);
}

fn drawn(screen: &SettingsScreen, layout: &Layout, now: Instant) -> Scene {
    let settings = test_settings();
    let look = view::tests::look(&settings, &settings);
    let mut scene = Scene::default();
    screen.draw(
        &mut scene,
        &look,
        (layout, &view::tests::frame(layout)),
        now,
    );
    scene
}

fn pane_texts(scene: &Scene) -> Vec<String> {
    scene
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
fn draws_the_band_the_hint_and_the_instrument_once_open() {
    let now = Instant::now();
    let screen = opened(now);
    let layout = view::tests::layout();

    let scene = drawn(&screen, &layout, after(now, 400));

    assert_eq!(scene.grounds.len(), 1);
    let texts = pane_texts(&scene);
    assert!(texts.contains(&Text::SettingModelHint.id().to_owned()));
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with(Text::SettingsRank.id()))
    );
    let hint = scene
        .panes
        .iter()
        .find(|pane| pane.lines[0].line.spans[0].text == Text::SettingModelHint.id())
        .unwrap();
    assert!(hint.lines[0].line.alpha > 0);
}

#[test]
fn leaves_the_instrument_out_of_a_window_without_a_gutter() {
    let now = Instant::now();
    let screen = opened(now);
    let narrow = Layout::reflowed(winit::dpi::PhysicalSize::new(1_200, 1_800), 2.0);
    assert!(!narrow.room().has_gutter());

    let scene = drawn(&screen, &narrow, after(now, 400));

    let texts = pane_texts(&scene);
    assert!(texts.contains(&Text::SettingModelHint.id().to_owned()));
    assert!(
        !texts
            .iter()
            .any(|text| text.starts_with(Text::SettingsRank.id()))
    );
}

#[test]
fn fades_in_what_it_draws_while_it_opens() {
    let now = Instant::now();
    let screen = opened(now);
    let layout = view::tests::layout();

    let opening = drawn(&screen, &layout, after(now, 100));
    let open = drawn(&screen, &layout, after(now, 400));

    let alpha = |scene: &Scene| scene.panes[0].lines[0].line.alpha;
    assert!(alpha(&opening) < alpha(&open));
    assert!(alpha(&opening) > 0);
}

#[test]
fn draws_nothing_once_closed() {
    let now = Instant::now();
    let mut screen = opened(now);
    screen.close(after(now, 400));
    let layout = view::tests::layout();

    let scene = drawn(&screen, &layout, after(now, 800));

    assert_eq!(scene.panes, []);
    assert_eq!(scene.grounds, []);
    assert_eq!(scene.marks, []);
}

#[test]
fn counts_every_setting_found_by_a_search() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();

    assert_eq!(screen.matching(&IdTranslator), 17);
    press(
        &mut screen,
        Gesture::Type("zzz".to_owned()),
        &mut settings,
        now,
    );
    assert_eq!(screen.matching(&IdTranslator), 0);
}

#[test]
fn draws_the_value_just_changed_warm_with_its_marks() {
    let now = Instant::now();
    let mut screen = opened(now);
    let mut settings = test_settings();
    press(&mut screen, Gesture::Left, &mut settings, now);
    let layout = view::tests::layout();
    let look = view::tests::look(&settings, &settings);
    let mut scene = Scene::default();

    let later = now + std::time::Duration::from_millis(400);
    screen.draw(
        &mut scene,
        &look,
        (&layout, &view::tests::frame(&layout)),
        later,
    );

    let warm = scene
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .any(|span| span.text == Model::SONNET_5_5.name && span.heat > 0);
    assert!(warm);
    assert_ne!(scene.marks.len(), 0);
}
