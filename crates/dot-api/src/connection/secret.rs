use std::fmt;

const MASKED: &str = "…";

/// Text that must never be shown: a key, a token, or what the user types of one.
///
/// `Debug` writes `…` in its place, so that no log, panic or error message carries it. Read it
/// with [`Secret::expose`], only where it is used.
///
/// # Examples
///
/// ```
/// use dot_api::Secret;
///
/// let secret = Secret::new("sk-ant-api03-abcdefghijklmnop");
///
/// assert_eq!(format!("{secret:?}"), "…");
/// assert_eq!(secret.expose(), "sk-ant-api03-abcdefghijklmnop");
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps `text`, which `Debug` no longer shows.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Returns the text itself.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Returns the text itself, to change it in place: what the user types is added to it.
    pub const fn expose_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(MASKED)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/connection/secret.rs"]
mod tests;
