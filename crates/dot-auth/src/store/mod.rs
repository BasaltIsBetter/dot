mod choice;
mod keychain;

use dot_api::ConnectError;

use crate::connector::{ConnectionStore, Remembered};

pub(crate) use self::choice::ChoiceFile;
pub(crate) use self::keychain::KeychainStore;

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
        if choice.holds_claude_code()? {
            return Ok(Some(Remembered::ClaudeCode));
        }

        let remembered = self.keys.load()?;
        if remembered == Some(Remembered::ClaudeCode) {
            choice.remember_claude_code()?;
            self.keys.forget()?;
        }
        Ok(remembered)
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        let Some(choice) = &self.choice else {
            return self.keys.save(remembered);
        };
        match remembered {
            Remembered::ClaudeCode => {
                choice.remember_claude_code()?;
                self.keys.forget()
            }
            Remembered::ApiKey(_) => {
                self.keys.save(remembered)?;
                choice.forget()
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/mod.rs"]
mod tests;
