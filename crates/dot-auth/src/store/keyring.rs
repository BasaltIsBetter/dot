use std::collections::HashMap;

use dot_api::{ConnectError, ConnectFailure};
use secret_service::blocking::{Item, SecretService};
use secret_service::{EncryptionType, Error};
use zbus::fdo;

use crate::connector::Remembered;
use crate::store::KeyStore;
use crate::store::stored::{ACCOUNT, SERVICE, decode, encode};

const SERVICE_ATTRIBUTE: &str = "service";
const ACCOUNT_ATTRIBUTE: &str = "account";
const LABEL: &str = "Dot";
const CONTENT_TYPE: &str = "text/plain";
const REPLACE: bool = true;

/// The desktop's keyring (GNOME Keyring, `KWallet`, `KeePassXC`), reached through the Secret
/// Service of the session bus.
///
/// A machine without one (a bare window manager, WSL) holds no key: there is nothing to load and
/// nothing to forget there, and only saving a key fails.
pub(crate) struct KeyringStore {
    service: &'static str,
    account: &'static str,
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self {
            service: SERVICE,
            account: ACCOUNT,
        }
    }
}

impl KeyringStore {
    fn attributes(&self) -> HashMap<&'static str, &'static str> {
        HashMap::from([
            (SERVICE_ATTRIBUTE, self.service),
            (ACCOUNT_ATTRIBUTE, self.account),
        ])
    }

    fn on_items<R>(&self, act: impl Fn(&Item<'_>) -> Result<R, Error>) -> Result<Vec<R>, Error> {
        let keyring = SecretService::connect(EncryptionType::Dh)?;
        let found = keyring.search_items(self.attributes())?;
        found
            .locked
            .iter()
            .inspect(|_| tracing::info!("the keyring asks to be unlocked"))
            .try_for_each(Item::unlock)?;
        found
            .unlocked
            .iter()
            .chain(&found.locked)
            .map(act)
            .collect()
    }
}

impl KeyStore for KeyringStore {
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method is tied to one lifetime"
    )]
    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        let secrets = self
            .on_items(|item| item.get_secret())
            .or_else(nothing_without_a_keyring)?;
        Ok(secrets.first().and_then(|stored| decode(stored)))
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        let store = || {
            let keyring = SecretService::connect(EncryptionType::Dh)?;
            let collection = keyring.get_default_collection()?;
            collection.unlock()?;
            collection
                .create_item(
                    LABEL,
                    self.attributes(),
                    encode(remembered).as_bytes(),
                    REPLACE,
                    CONTENT_TYPE,
                )
                .map(drop)
        };
        store().map_err(storage_failed)
    }

    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method is tied to one lifetime"
    )]
    fn forget(&self) -> Result<(), ConnectError> {
        self.on_items(|item| item.delete())
            .or_else(nothing_without_a_keyring)
            .map(drop)
    }
}

fn nothing_without_a_keyring<R>(error: Error) -> Result<Vec<R>, ConnectError> {
    if has_no_keyring(&error) {
        tracing::info!("this session has no keyring: no API key can be kept");
        return Ok(Vec::new());
    }
    Err(storage_failed(error))
}

fn has_no_keyring(error: &Error) -> bool {
    match error {
        Error::Unavailable | Error::Zbus(zbus::Error::Connection(..) | zbus::Error::Address(_)) => {
            true
        }
        Error::Zbus(zbus::Error::FDO(refusal)) => matches!(
            **refusal,
            fdo::Error::ServiceUnknown(_) | fdo::Error::NameHasNoOwner(_)
        ),
        _ => false,
    }
}

fn storage_failed(error: Error) -> ConnectError {
    ConnectError::caused_by(ConnectFailure::StorageFailed, error)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/keyring.rs"]
mod tests;
