use dot_api::{ConnectError, ConnectFailure};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::connector::Remembered;
use crate::store::KeyStore;
use crate::store::stored::{ACCOUNT, SERVICE, decode, encode};

const ITEM_NOT_FOUND: i32 = -25300;

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

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/keychain.rs"]
mod tests;
