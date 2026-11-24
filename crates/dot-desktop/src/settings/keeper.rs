use std::sync::{Arc, Mutex, PoisonError};

use dot_api::{Preferences, PreferencesError, Settings};

use crate::worker::{Mailbox, Workers};

const KEEPER_FELL: &str = "the settings writer stopped";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SettingsNews {
    Saved,
    Unsaved(PreferencesError),
}

pub(crate) struct Keeper<F, M> {
    preferences: Arc<F>,
    defaults: Settings,
    workers: Workers<M>,
    written: Arc<Mutex<u64>>,
    latest: u64,
}

impl<F: Preferences + 'static, M: Mailbox<SettingsNews>> Keeper<F, M> {
    pub(crate) fn new(preferences: F, defaults: Settings, mailbox: M) -> Self {
        Self {
            preferences: Arc::new(preferences),
            defaults,
            workers: Workers::new(mailbox),
            written: Arc::default(),
            latest: 0,
        }
    }

    pub(crate) const fn defaults(&self) -> &Settings {
        &self.defaults
    }

    pub(crate) fn keep(&mut self, settings: Settings) {
        self.latest += 1;
        let generation = self.latest;
        let preferences = Arc::clone(&self.preferences);
        let written = Arc::clone(&self.written);
        let defaults = self.defaults;
        self.workers.spawn(
            move |_| {
                save_unless_outdated(&*preferences, (&settings, &defaults), &written, generation)
            },
            || {
                Some(SettingsNews::Unsaved(PreferencesError(
                    KEEPER_FELL.to_owned(),
                )))
            },
        );
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }
}

fn save_unless_outdated(
    preferences: &impl Preferences,
    (settings, defaults): (&Settings, &Settings),
    written: &Mutex<u64>,
    generation: u64,
) -> SettingsNews {
    let mut last = written.lock().unwrap_or_else(PoisonError::into_inner);
    if *last > generation {
        return SettingsNews::Saved;
    }
    *last = generation;
    match preferences.save(settings, defaults) {
        Ok(()) => SettingsNews::Saved,
        Err(error) => {
            tracing::warn!(%error, "the settings were not saved");
            SettingsNews::Unsaved(error)
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/settings/keeper.rs"]
mod tests;
