use std::error::Error;
use std::fmt;

use crate::connection::api_key::ApiKey;
use crate::connection::secret::Secret;

const SHORTEST: usize = 20;
const LONGEST: usize = 4_096;

/// Short-lived OAuth access token of an Anthropic account, as `ant` hands it out.
///
/// `Debug` never shows the token: read it with [`AccessToken::expose`], only where it leaves for
/// Anthropic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessToken(Secret);

/// Reason a text is not shaped like an access token: not 20 to 4,096 visible ASCII characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessTokenError;

/// What authenticates Dot's requests to Claude, for the current connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Credential {
    /// An API key, sent in the `x-api-key` header.
    ApiKey(ApiKey),
    /// An Anthropic account's access token, sent as a bearer token.
    AccessToken(AccessToken),
    /// Nothing to send: requests go through Claude Code, which signs in on its own.
    ClaudeCode,
}

impl AccessToken {
    /// Reads an access token, ignoring the spaces and line breaks around it.
    ///
    /// Only the shape is checked: Anthropic alone knows whether the token is valid.
    ///
    /// # Errors
    ///
    /// Returns [`AccessTokenError`] if the text is not 20 to 4,096 characters long, or holds
    /// anything but visible ASCII characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::AccessToken;
    ///
    /// assert!(AccessToken::parse("sk-ant-oat01-abcdefghijklmnop\n").is_ok());
    /// assert!(AccessToken::parse("not a token").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, AccessTokenError> {
        let token = text.trim();
        let well_shaped = (SHORTEST..=LONGEST).contains(&token.len())
            && token.bytes().all(|byte| byte.is_ascii_graphic());
        if !well_shaped {
            return Err(AccessTokenError);
        }
        Ok(Self(Secret::new(token)))
    }

    /// Returns the token itself, to send it to Anthropic.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose()
    }
}

impl fmt::Display for AccessTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the text is not shaped like an access token")
    }
}

impl Error for AccessTokenError {}

#[cfg(test)]
#[path = "../../../../tests/dot-api/connection/credential.rs"]
mod tests;
