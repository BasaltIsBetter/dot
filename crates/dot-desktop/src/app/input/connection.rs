use std::path::{Path, PathBuf};
use std::time::Instant;

use dot_api::{ApiKey, Connection, Text, Translator};

use crate::app::{DesktopApp, Parts, Screen};
use crate::settings::{Actions, Setting};
use crate::shell::choose_folder;
use crate::sign_in::Report;

const KEY_DOTS: &str = "••••";
const KEY_ENDING: usize = 4;

impl<P: Parts> DesktopApp<'_, P> {
    pub(in crate::app) fn act(&mut self, setting: Setting, now: Instant) {
        match setting {
            Setting::Folder => {
                let prompt = self.translator.translate(Text::FolderPickerPrompt);
                choose_folder(&prompt, self.mailbox.clone());
            }
            Setting::Account => {
                self.close_settings(now);
                self.sign_in.change();
                self.screen = Screen::SignIn;
            }
            Setting::SignOut => self.sign_in.sign_out(),
            _ => {}
        }
    }

    pub(in crate::app) fn try_key(&mut self, key: ApiKey) {
        self.key_check = Some(shown_key(&key));
        self.sign_in.replace_key(key);
    }

    pub(in crate::app) fn connection_news(&mut self, report: &Report, now: Instant) {
        match report {
            Report::KeyReplaced(Ok(())) => self.key_shown = self.key_check.take(),
            Report::KeyReplaced(Err(error)) | Report::SignedOut(Err(error)) => {
                self.key_check = None;
                let notice = self.translator.translate(error.failure().text());
                self.sessions.active_mut().conversation.notice(&notice, now);
            }
            Report::SignedOut(Ok(())) => {
                self.key_shown = None;
                self.close_settings(now);
                self.screen = Screen::SignIn;
            }
            _ => {}
        }
    }

    pub(in crate::app) fn actions(&self) -> Actions {
        let translator = self.translator;
        let folder = self.sessions.active().folder.as_deref().map_or_else(
            || translator.translate(Text::FolderNone).into_owned(),
            |folder| folder.display().to_string(),
        );
        let connection = self.sign_in.connection();
        let account = match connection {
            Some(Connection::ClaudeCode) => Text::SignInClaudeAccount,
            Some(Connection::AnthropicAccount) => Text::SignInAnthropicAccount,
            Some(Connection::ApiKey) => Text::SignInApiKey,
            None => Text::AccountNone,
        };
        let key = match (&self.key_check, connection, &self.key_shown) {
            (Some(_), _, _) => translator.translate(Text::ApiKeyChecking).into_owned(),
            (None, Some(Connection::ApiKey), Some(shown)) => shown.clone(),
            (None, Some(Connection::ApiKey), None) => {
                translator.translate(Text::ApiKeyKept).into_owned()
            }
            _ => translator.translate(Text::ApiKeyNone).into_owned(),
        };
        Actions {
            folder,
            account: translator.translate(account).into_owned(),
            key,
        }
    }

    pub(in crate::app) fn open_session_in(&mut self, folder: Option<PathBuf>) {
        let parts = (self.open)(folder.clone());
        let answering = (self.settings.permissions, self.settings.tuning);
        self.sessions
            .open(parts, &self.connector, (answering, &self.mailbox));
        self.sessions.active_mut().folder = folder;
        self.refresh_status();
    }

    pub(in crate::app) fn in_session_folder(&self, path: String) -> String {
        let folder = self.sessions.active().folder.as_deref();
        if folder == self.launch_folder.as_deref() || Path::new(&path).is_absolute() {
            return path;
        }
        folder.map_or(path.clone(), |folder| {
            folder.join(&path).display().to_string()
        })
    }
}

fn shown_key(key: &ApiKey) -> String {
    let typed = key.expose();
    let count = typed.chars().count();
    let ending: String = typed
        .chars()
        .skip(count.saturating_sub(KEY_ENDING))
        .collect();
    format!("{KEY_DOTS} {ending}")
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/connection.rs"]
mod tests;
