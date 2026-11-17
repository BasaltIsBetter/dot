use std::fmt;

const PREFIX: &str = "sk-ant-";
const SHORTEST: usize = 20;
const LONGEST: usize = 256;

/// Anthropic API key, checked for shape when it enters Dot.
///
/// `Debug` never shows the key: read it with [`ApiKey::expose`], only where it leaves for
/// Anthropic or the keychain.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

/// Reason a text is not shaped like an Anthropic API key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiKeyError {
    /// The text does not start with `sk-ant-`.
    MissingPrefix,
    /// The text is shorter than 20 or longer than 256 characters.
    WrongLength,
    /// The text holds a character other than an ASCII letter, a digit, `-` or `_`.
    InvalidCharacter,
}

impl ApiKey {
    /// Reads an API key, ignoring the spaces and line breaks around it.
    ///
    /// Only the shape is checked: Anthropic alone knows whether the key is valid.
    ///
    /// # Errors
    ///
    /// Returns [`ApiKeyError`] if the text does not start with `sk-ant-`, is not 20 to 256
    /// characters long, or holds anything but ASCII letters, digits, `-` and `_`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::{ApiKey, ApiKeyError};
    ///
    /// assert!(ApiKey::parse(" sk-ant-api03-abcdefghijklmnop\n").is_ok());
    /// assert_eq!(ApiKey::parse("hello"), Err(ApiKeyError::MissingPrefix));
    /// ```
    pub fn parse(text: &str) -> Result<Self, ApiKeyError> {
        let key = text.trim();
        if !key.starts_with(PREFIX) {
            return Err(ApiKeyError::MissingPrefix);
        }
        if !(SHORTEST..=LONGEST).contains(&key.len()) {
            return Err(ApiKeyError::WrongLength);
        }
        if !key.bytes().all(is_key_byte) {
            return Err(ApiKeyError::InvalidCharacter);
        }
        Ok(Self(key.to_owned()))
    }

    /// Returns the key itself, to send it to Anthropic or store it in the keychain.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ApiKey({PREFIX}…)")
    }
}

fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

#[cfg(test)]
#[path = "../../../tests/dot-api/api_key.rs"]
mod tests;
