use dot_api::ApiKey;

use crate::connector::Remembered;
use crate::store::{ANTHROPIC_ACCOUNT, CLAUDE_CODE, secretless_named};

pub(crate) const SERVICE: &str = "dev.dot.desktop";
pub(crate) const ACCOUNT: &str = "claude-connection";
const API_KEY_PREFIX: &str = "api-key:";

pub(crate) fn encode(remembered: &Remembered) -> String {
    match remembered {
        Remembered::ApiKey(key) => format!("{API_KEY_PREFIX}{}", key.expose()),
        Remembered::ClaudeCode => CLAUDE_CODE.to_owned(),
        Remembered::AnthropicAccount => ANTHROPIC_ACCOUNT.to_owned(),
    }
}

pub(crate) fn decode(stored: &[u8]) -> Option<Remembered> {
    let text = std::str::from_utf8(stored).ok()?;
    if let Some(secretless) = secretless_named(text) {
        return Some(secretless);
    }

    let key = text.strip_prefix(API_KEY_PREFIX)?;
    ApiKey::parse(key).ok().map(Remembered::ApiKey)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/store/stored.rs"]
mod tests;
