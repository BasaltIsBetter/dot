use std::borrow::Cow;
use std::time::Duration;

use dot_api::{Language, Text};

use super::*;
use crate::settings::catalog::OutputPart;
use crate::settings::tests::test_settings;

struct Labels;

impl Translator for Labels {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(match text {
            Text::SettingsConversation => "Conversation",
            Text::SettingsDisplay => "Affichage",
            Text::SettingModel => "Modèle",
            Text::SettingEffort => "Effort",
            Text::SettingColor => "Couleur",
            Text::SettingColumn => "Largeur de colonne",
            other => other.id(),
        })
    }

    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }

    fn use_language(&self, _language: Language) {}
}

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn starts_on_the_model() {
    let reel = Reel::new();

    assert_eq!(reel.focus(), Setting::Model);
    assert_eq!(reel.focus_index(), 1);
    assert_eq!(reel.rank(), (1, 22));
}

#[test]
fn steps_over_the_section_titles() {
    let now = Instant::now();
    let mut reel = Reel::new();
    for _ in 0..5 {
        reel.step(Step::Down, &Labels, now);
    }

    assert_eq!(reel.focus(), Setting::Color);
    assert_eq!(reel.focus_index(), 7);
    assert_eq!(reel.rank(), (6, 22));
    reel.step(Step::Up, &Labels, now);
    assert_eq!(reel.focus(), Setting::Folder);
}

#[test]
fn stays_on_the_first_and_the_last_setting() {
    let now = Instant::now();
    let mut reel = Reel::new();

    reel.step(Step::Up, &Labels, now);
    assert_eq!(reel.focus(), Setting::Model);
    for _ in 0..30 {
        reel.step(Step::Down, &Labels, now);
    }
    assert_eq!(reel.focus(), Setting::SignOut);
    assert_eq!(reel.rank(), (22, 22));
}

#[test]
fn glides_toward_the_setting_in_focus() {
    let now = Instant::now();
    let mut reel = Reel::new();

    reel.step(Step::Down, &Labels, now);

    assert!(reel.shift.is_moving(after(now, 100)));
    assert!(reel.shift.position(after(now, 100)) > 1.0);
    assert!((reel.shift.position(after(now, 1_000)) - 2.0).abs() < f32::EPSILON);
}

#[test]
fn opens_on_the_setting_asked_for_and_forgets_the_search() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.type_query("eff", &Labels, now);

    reel.open(Some(Setting::Column), None, now);

    assert_eq!(reel.focus(), Setting::Column);
    assert_eq!(reel.query(), "");
}

#[test]
fn opens_where_it_was_left_when_no_setting_is_asked_for() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.step(Step::Down, &Labels, now);

    reel.open(None, None, now);

    assert_eq!(reel.focus(), Setting::Effort);
}

#[test]
fn finds_a_setting_by_its_name_whatever_its_accents_and_case() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.step(Step::Down, &Labels, now);

    reel.type_query("MODELE", &Labels, now);

    assert_eq!(reel.focus(), Setting::Model);
    assert_eq!(reel.matching(&Labels), 1);
    assert!(reel.matches(1, &Labels));
    assert!(!reel.matches(2, &Labels));
}

#[test]
fn finds_every_setting_of_a_section_by_the_section_s_name() {
    let now = Instant::now();
    let mut reel = Reel::new();

    reel.type_query("affich", &Labels, now);

    assert_eq!(reel.focus(), Setting::Color);
    assert_eq!(reel.matching(&Labels), 9);
}

#[test]
fn never_matches_a_section_title_itself() {
    let mut reel = Reel::new();
    reel.type_query("conv", &Labels, Instant::now());

    assert!(!reel.matches(0, &Labels));
    assert!(reel.matches(1, &Labels));
}

#[test]
fn steps_only_through_the_settings_found() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.type_query("co", &Labels, now);
    assert_eq!(reel.focus(), Setting::Model);
    let mut visited = vec![reel.focus()];

    for _ in 0..22 {
        reel.step(Step::Down, &Labels, now);
        visited.push(reel.focus());
    }
    visited.dedup();

    assert_eq!(
        visited,
        [
            Setting::Model,
            Setting::Effort,
            Setting::Summaries,
            Setting::Permissions,
            Setting::Folder,
            Setting::Color,
            Setting::StateColors,
            Setting::ToolColors,
            Setting::Column,
            Setting::SessionsColumn,
            Setting::Output(OutputPart::ContentsCut),
            Setting::Account,
            Setting::ApiKey,
            Setting::SignOut,
        ]
    );
}

#[test]
fn keeps_the_focus_when_it_still_matches() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.step(Step::Down, &Labels, now);

    reel.type_query("eff", &Labels, now);

    assert_eq!(reel.focus(), Setting::Effort);
}

#[test]
fn keeps_the_focus_when_nothing_matches() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.step(Step::Down, &Labels, now);

    reel.type_query("zzz", &Labels, now);

    assert_eq!(reel.focus(), Setting::Effort);
    assert_eq!(reel.matching(&Labels), 0);
}

#[test]
fn erases_the_search_one_character_at_a_time() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.type_query("ef", &Labels, now);

    assert!(reel.erase_query(&Labels, now));
    assert_eq!(reel.query(), "e");
    assert!(reel.erase_query(&Labels, now));
    assert!(!reel.erase_query(&Labels, now));
    assert_eq!(reel.matching(&Labels), 22);
}

#[test]
fn clears_the_search_and_says_whether_there_was_one() {
    let mut reel = Reel::new();
    assert!(!reel.clear_query());

    reel.type_query("e", &Labels, Instant::now());

    assert!(reel.clear_query());
    assert_eq!(reel.query(), "");
}

#[test]
fn steps_once_for_each_forty_points_of_wheel() {
    let now = Instant::now();
    let mut reel = Reel::new();

    reel.wheel(39.0, &Labels, now);
    assert_eq!(reel.focus(), Setting::Model);
    reel.wheel(2.0, &Labels, after(now, 300));
    assert_eq!(reel.focus(), Setting::Effort);
    reel.wheel(81.0, &Labels, after(now, 600));
    assert_eq!(reel.focus(), Setting::Permissions);
    reel.wheel(-160.0, &Labels, after(now, 900));
    assert_eq!(reel.focus(), Setting::Model);
}

#[test]
fn lets_the_wheel_that_opened_the_settings_end_before_it_steps() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.open(None, Some(now), now);

    reel.wheel(400.0, &Labels, after(now, 100));
    reel.wheel(400.0, &Labels, after(now, 250));
    assert_eq!(reel.focus(), Setting::Model);

    reel.wheel(40.0, &Labels, after(now, 500));
    assert_eq!(reel.focus(), Setting::Effort);
}

#[test]
fn steps_with_the_wheel_at_once_when_opened_another_way() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.open(None, None, now);

    reel.wheel(40.0, &Labels, after(now, 10));

    assert_eq!(reel.focus(), Setting::Effort);
}

#[test]
fn tells_each_setting_changed_since_it_opened_once() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.open(None, None, now);
    reel.changed(Setting::Effort);
    reel.changed(Setting::Model);
    reel.changed(Setting::Effort);

    assert_eq!(reel.close(), [Setting::Effort, Setting::Model]);
    reel.open(None, None, now);
    assert_eq!(reel.close(), []);
}

#[test]
fn forgets_the_search_when_it_closes() {
    let mut reel = Reel::new();
    reel.type_query("eff", &Labels, Instant::now());

    reel.close();

    assert_eq!(reel.query(), "");
}

#[test]
fn tells_a_setting_changed_from_its_default() {
    let mut settings = test_settings();
    assert!(!is_changed(Setting::Column, &settings, &test_settings()));

    Setting::Column.choose(0, &mut settings);

    assert!(is_changed(Setting::Column, &settings, &test_settings()));
    assert!(!is_changed(Setting::Model, &settings, &test_settings()));
}

#[test]
fn places_each_setting_in_the_list() {
    assert_eq!(place_of(Setting::Model), 1);
    assert_eq!(place_of(Setting::Permissions), 4);
    assert_eq!(place_of(Setting::Folder), 5);
    assert_eq!(place_of(Setting::Color), 7);
    assert_eq!(place_of(Setting::SessionsColumn), 15);
    assert_eq!(place_of(Setting::Output(OutputPart::Cleaning)), 17);
    assert_eq!(place_of(Setting::SignOut), 25);
}

#[test]
fn folds_accents_and_case() {
    assert_eq!(folded("ÉtÉ À Çà ôù ÏÿÁ"), "ete a ca ou iya");
}

#[test]
fn a_wheel_two_hundred_milliseconds_after_the_opening_one_is_a_new_gesture() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.open(None, Some(now), now);

    reel.wheel(40.0, &Labels, after(now, 200));

    assert_eq!(reel.focus(), Setting::Effort);
}
