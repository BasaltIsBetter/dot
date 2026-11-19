mod choice;
mod keychain;

use dot_api::ConnectError;

use crate::connector::{ConnectionStore, Remembered};

pub(crate) use self::choice::ChoiceFile;
pub(crate) use self::keychain::KeychainStore;

const CLAUDE_CODE: &str = "claude-code";
const ANTHROPIC_ACCOUNT: &str = "anthropic-account";

pub(crate) trait KeyStore: Send + Sync {
    fn load(&self) -> Result<Option<Remembered>, ConnectError>;

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError>;

    fn forget(&self) -> Result<(), ConnectError>;
}

pub(crate) struct SplitStore<K> {
    choice: Option<ChoiceFile>,
    keys: K,
}

impl<K: KeyStore> SplitStore<K> {
    pub(crate) const fn new(choice: Option<ChoiceFile>, keys: K) -> Self {
        Self { choice, keys }
    }
}

impl<K: KeyStore> ConnectionStore for SplitStore<K> {
    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        let Some(choice) = &self.choice else {
            return self.keys.load();
        };
        if let Some(remembered) = choice.choice()? {
            return Ok(Some(remembered));
        }

        let remembered = self.keys.load()?;
        if remembered == Some(Remembered::ClaudeCode) {
            choice.remember(&Remembered::ClaudeCode)?;
            self.keys.forget()?;
        }
        Ok(remembered)
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        let Some(choice) = &self.choice else {
            return self.keys.save(remembered);
        };
        match remembered {
            Remembered::ClaudeCode | Remembered::AnthropicAccount => {
                choice.remember(remembered)?;
                self.keys.forget()
            }
            Remembered::ApiKey(_) => {
                self.keys.save(remembered)?;
                choice.forget()
            }
        }
    }
}

const fn secretless_name(remembered: &Remembered) -> Option<&'static str> {
    match remembered {
        Remembered::ClaudeCode => Some(CLAUDE_CODE),
        Remembered::AnthropicAccount => Some(ANTHROPIC_ACCOUNT),
        Remembered::ApiKey(_) => None,
    }
}

fn secretless_named(name: &str) -> Option<Remembered> {
    match name {
        CLAUDE_CODE => Some(Remembered::ClaudeCode),
        ANTHROPIC_ACCOUNT => Some(Remembered::AnthropicAccount),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/mod.rs"]
mod tests;
