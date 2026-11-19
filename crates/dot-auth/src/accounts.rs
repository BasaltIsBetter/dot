use std::sync::{Mutex, PoisonError};

use dot_api::{
    ApiKey, Cancellation, ConnectError, ConnectFailure, Connection, Connector, Credential,
};

use crate::connector::{
    AnthropicAccount, ClaudeCodeAccount, ConnectionStore, KeyVerifier, Remembered, Verdict,
};

pub(crate) struct Accounts<S, V, C, A> {
    store: S,
    verifier: V,
    claude_code: Option<C>,
    ant: Option<A>,
    remembered: Mutex<Option<Remembered>>,
}

impl<S, V, C, A> Accounts<S, V, C, A>
where
    S: ConnectionStore,
    V: KeyVerifier,
    C: ClaudeCodeAccount,
    A: AnthropicAccount,
{
    pub(crate) const fn new(store: S, verifier: V, claude_code: Option<C>, ant: Option<A>) -> Self {
        Self {
            store,
            verifier,
            claude_code,
            ant,
            remembered: Mutex::new(None),
        }
    }

    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        let remembered = self.store.load()?;
        self.keep(remembered.clone());
        Ok(remembered)
    }

    fn remembered(&self) -> Result<Option<Remembered>, ConnectError> {
        let kept = self
            .remembered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        match kept {
            Some(remembered) => Ok(Some(remembered)),
            None => self.load(),
        }
    }

    fn remember(&self, remembered: Remembered) -> Result<(), ConnectError> {
        self.store.save(&remembered)?;
        self.keep(Some(remembered));
        Ok(())
    }

    fn keep(&self, remembered: Option<Remembered>) {
        *self
            .remembered
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = remembered;
    }

    fn ant(&self) -> Result<&A, ConnectError> {
        self.ant
            .as_ref()
            .ok_or_else(|| ConnectError::new(ConnectFailure::AntMissing))
    }
}

impl<S, V, C, A> Connector for Accounts<S, V, C, A>
where
    S: ConnectionStore,
    V: KeyVerifier,
    C: ClaudeCodeAccount,
    A: AnthropicAccount,
{
    fn current(&self) -> Result<Option<Connection>, ConnectError> {
        match self.load()? {
            None => Ok(None),
            Some(Remembered::ApiKey(_)) => Ok(Some(Connection::ApiKey)),
            Some(Remembered::ClaudeCode) => {
                let Some(claude_code) = &self.claude_code else {
                    return Ok(None);
                };
                Ok(claude_code
                    .is_signed_in()?
                    .then_some(Connection::ClaudeCode))
            }
            Some(Remembered::AnthropicAccount) => {
                let signed_in = self
                    .ant
                    .as_ref()
                    .is_some_and(|ant| ant.access_token().is_ok());
                Ok(signed_in.then_some(Connection::AnthropicAccount))
            }
        }
    }

    fn connect_with_api_key(&self, key: &ApiKey) -> Result<(), ConnectError> {
        match self.verifier.verify(key) {
            Verdict::Accepted => self.remember(Remembered::ApiKey(key.clone())),
            Verdict::Rejected => Err(ConnectError::new(ConnectFailure::KeyRejected)),
            Verdict::Unreachable(cause) => Err(ConnectError::caused_by(
                ConnectFailure::AnthropicUnreachable,
                cause,
            )),
        }
    }

    fn connect_with_claude_code(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        let claude_code = self
            .claude_code
            .as_ref()
            .ok_or_else(|| ConnectError::new(ConnectFailure::ClaudeCodeMissing))?;

        if !claude_code.is_signed_in()? {
            claude_code.sign_in(cancellation)?;
            if !claude_code.is_signed_in()? {
                return Err(ConnectError::new(ConnectFailure::ClaudeCodeFailed));
            }
        }

        self.remember(Remembered::ClaudeCode)
    }

    fn connect_with_anthropic_account(
        &self,
        cancellation: &Cancellation,
    ) -> Result<(), ConnectError> {
        let ant = self.ant()?;

        if ant.access_token().is_err() {
            ant.sign_in(cancellation)?;
            ant.access_token()?;
        }

        self.remember(Remembered::AnthropicAccount)
    }

    fn credential(&self) -> Result<Credential, ConnectError> {
        match self.remembered()? {
            Some(Remembered::ApiKey(key)) => Ok(Credential::ApiKey(key)),
            Some(Remembered::AnthropicAccount) => {
                self.ant()?.access_token().map(Credential::AccessToken)
            }
            Some(Remembered::ClaudeCode) => Ok(Credential::ClaudeCode),
            None => Err(ConnectError::new(ConnectFailure::NotConnected)),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-auth/accounts.rs"]
mod tests;
