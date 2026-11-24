use dot_api::{ApiKey, Secret, Text};

use crate::settings::catalog::{Kind, Setting};
use crate::settings::{Gesture, Outcome};

const LONGEST_KEY: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Waiting {
    #[default]
    Nothing,
    SignOut,
    Key(usize),
}

#[derive(Debug, Default)]
pub(crate) struct Pending {
    key: Option<Secret>,
    confirming: bool,
}

impl Pending {
    pub(crate) fn waiting(&self) -> Waiting {
        match &self.key {
            Some(draft) => Waiting::Key(draft.expose().chars().count()),
            None if self.confirming => Waiting::SignOut,
            None => Waiting::Nothing,
        }
    }

    pub(crate) const fn is_typing_key(&self) -> bool {
        self.key.is_some()
    }

    pub(crate) fn clear(&mut self) {
        self.key = None;
        self.confirming = false;
    }

    pub(crate) fn gesture(&mut self, focus: Setting, gesture: &Gesture) -> Option<Outcome> {
        if let Some(draft) = self.key.as_mut() {
            return Some(match gesture {
                Gesture::Type(typed) => {
                    let room = LONGEST_KEY.saturating_sub(draft.expose().len());
                    let kept = typed.chars().filter(char::is_ascii_graphic).take(room);
                    draft.expose_mut().extend(kept);
                    Outcome::Nothing
                }
                Gesture::Erase => {
                    draft.expose_mut().pop();
                    Outcome::Nothing
                }
                Gesture::Enter => match ApiKey::parse(draft.expose()) {
                    Ok(key) => {
                        self.key = None;
                        Outcome::KeyTyped(key)
                    }
                    Err(_) => Outcome::Refused(Text::SignInKeyMalformed),
                },
                _ => {
                    self.key = None;
                    Outcome::Nothing
                }
            });
        }
        let confirming = std::mem::take(&mut self.confirming);
        if focus.kind() != Kind::Action {
            return None;
        }
        match gesture {
            Gesture::Enter => Some(match focus {
                Setting::SignOut if confirming => Outcome::Act(Setting::SignOut),
                Setting::SignOut => {
                    self.confirming = true;
                    Outcome::Nothing
                }
                Setting::ApiKey => {
                    self.key = Some(Secret::with_capacity(LONGEST_KEY));
                    Outcome::Nothing
                }
                other => Outcome::Act(other),
            }),
            Gesture::Left | Gesture::Right => Some(Outcome::Nothing),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/settings/action.rs"]
mod tests;
