use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::Column;

use super::*;
use crate::settings::tests::{MemoryPreferences, REFUSED, test_settings};

const PATIENCE: Duration = Duration::from_secs(5);

struct FallingPreferences;

impl Preferences for FallingPreferences {
    fn load(&self, defaults: &Settings) -> Settings {
        *defaults
    }

    fn save(&self, _settings: &Settings, _defaults: &Settings) -> Result<(), PreferencesError> {
        panic!("the disk fell");
    }
}

fn keeper<F: Preferences + 'static>(
    preferences: F,
) -> (Keeper<F, Sender<SettingsNews>>, Receiver<SettingsNews>) {
    let (sender, receiver) = mpsc::channel();
    (Keeper::new(preferences, test_settings(), sender), receiver)
}

fn narrow() -> Settings {
    Settings {
        column: Column::Narrow,
        ..test_settings()
    }
}

#[test]
fn keeps_the_settings_off_the_interface_thread_and_says_so() {
    let preferences = MemoryPreferences::default();
    let (mut keeper, receiver) = keeper(preferences.clone());

    keeper.keep(narrow());

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(SettingsNews::Saved));
    assert_eq!(*preferences.saved.lock().unwrap(), [narrow()]);
    keeper.stop();
}

#[test]
fn says_why_the_settings_were_not_kept() {
    let preferences = MemoryPreferences {
        refuses: true,
        ..MemoryPreferences::default()
    };
    let (mut keeper, receiver) = keeper(preferences);

    keeper.keep(narrow());

    assert_eq!(
        receiver.recv_timeout(PATIENCE),
        Ok(SettingsNews::Unsaved(PreferencesError(REFUSED.to_owned())))
    );
    keeper.stop();
}

#[test]
fn says_the_settings_were_not_kept_when_the_writer_falls() {
    let (mut keeper, receiver) = keeper(FallingPreferences);

    keeper.keep(narrow());

    assert_eq!(
        receiver.recv_timeout(PATIENCE),
        Ok(SettingsNews::Unsaved(PreferencesError(
            KEEPER_FELL.to_owned()
        )))
    );
    keeper.stop();
}

#[test]
fn waits_for_the_last_settings_to_be_kept_when_it_stops() {
    let preferences = MemoryPreferences::default();
    let (mut keeper, _receiver) = keeper(preferences.clone());

    keeper.keep(test_settings());
    keeper.keep(narrow());
    keeper.stop();

    assert_eq!(preferences.saved.lock().unwrap().last(), Some(&narrow()));
}

#[test]
fn never_writes_older_settings_over_newer_ones() {
    let preferences = MemoryPreferences::default();
    let written = Mutex::new(5);

    let news = save_unless_outdated(&preferences, (&narrow(), &test_settings()), &written, 4);

    assert_eq!(news, SettingsNews::Saved);
    assert!(preferences.saved.lock().unwrap().is_empty());
    assert_eq!(*written.lock().unwrap(), 5);
}

#[test]
fn writes_the_settings_of_the_same_or_a_newer_change() {
    let preferences = MemoryPreferences::default();
    let written = Mutex::new(5);

    save_unless_outdated(&preferences, (&narrow(), &test_settings()), &written, 5);
    save_unless_outdated(
        &preferences,
        (&test_settings(), &test_settings()),
        &written,
        6,
    );

    assert_eq!(
        *preferences.saved.lock().unwrap(),
        [narrow(), test_settings()]
    );
    assert_eq!(*written.lock().unwrap(), 6);
}

#[test]
fn hands_back_dot_s_defaults() {
    let (keeper, _receiver) = keeper(MemoryPreferences::default());

    assert_eq!(keeper.defaults(), &test_settings());
}

#[test]
fn numbers_each_change_it_keeps() {
    let (mut keeper, _receiver) = keeper(MemoryPreferences::default());

    keeper.keep(narrow());
    keeper.keep(test_settings());
    keeper.keep(narrow());

    assert_eq!(keeper.latest, 3);
    keeper.stop();
}
