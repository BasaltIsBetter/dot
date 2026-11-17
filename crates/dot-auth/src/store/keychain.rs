use dot_api::{ApiKey, ConnectError, ConnectFailure};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::connector::Remembered;
use crate::store::KeyStore;

const SERVICE: &str = "dev.dot.desktop";
const ACCOUNT: &str = "claude-connection";
const ITEM_NOT_FOUND: i32 = -25300;
const CLAUDE_CODE: &str = "claude-code";
const API_KEY_PREFIX: &str = "api-key:";

pub(crate) struct KeychainStore {
    service: &'static str,
    account: &'static str,
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self {
            service: SERVICE,
            account: ACCOUNT,
        }
    }
}

impl KeyStore for KeychainStore {
    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        match get_generic_password(self.service, self.account) {
            Ok(stored) => Ok(decode(&stored)),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(ConnectError::caused_by(
                ConnectFailure::StorageFailed,
                error,
            )),
        }
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        set_generic_password(self.service, self.account, encode(remembered).as_bytes())
            .map_err(|error| ConnectError::caused_by(ConnectFailure::StorageFailed, error))
    }

    fn forget(&self) -> Result<(), ConnectError> {
        match delete_generic_password(self.service, self.account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
            Err(error) => Err(ConnectError::caused_by(
                ConnectFailure::StorageFailed,
                error,
            )),
        }
    }
}

fn encode(remembered: &Remembered) -> String {
    match remembered {
        Remembered::ClaudeCode => CLAUDE_CODE.to_owned(),
        Remembered::ApiKey(key) => format!("{API_KEY_PREFIX}{}", key.expose()),
    }
}

fn decode(stored: &[u8]) -> Option<Remembered> {
    let text = std::str::from_utf8(stored).ok()?;
    if text == CLAUDE_CODE {
        return Some(Remembered::ClaudeCode);
    }

    let key = text.strip_prefix(API_KEY_PREFIX)?;
    ApiKey::parse(key).ok().map(Remembered::ApiKey)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/keychain.rs"]
mod tests;
