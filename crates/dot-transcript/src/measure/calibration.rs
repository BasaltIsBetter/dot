const PRIOR_CHARACTERS_PER_TOKEN: f64 = 2.3;

/// How many tokens the characters Claude shows are worth, learned from the conversation's own
/// replies: the tokens Claude counted for them over the characters they showed.
///
/// Before the first reply has been counted, a character is worth 1 / 2.3 of a token: the
/// median of 664 replies of Claude, text only, measured on one user's history on 2026-11-21.
/// What Claude writes without showing it (its full thinking, the input of its tools) ends up
/// in the learned value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Calibration {
    characters: u64,
    tokens: u64,
}

impl Calibration {
    /// Returns how many tokens `characters` shown characters are worth.
    #[must_use]
    pub fn tokens_for(&self, characters: usize) -> f64 {
        let characters = to_f64(u64::try_from(characters).unwrap_or(u64::MAX));
        if self.characters == 0 {
            return characters / PRIOR_CHARACTERS_PER_TOKEN;
        }
        characters * to_f64(self.tokens) / to_f64(self.characters)
    }

    /// Learns from a reply that showed `characters` characters and wrote `tokens` tokens; a
    /// reply that showed or wrote nothing teaches nothing.
    pub fn learn(&mut self, characters: u64, tokens: u64) {
        if characters == 0 || tokens == 0 {
            return;
        }
        self.characters = self.characters.saturating_add(characters);
        self.tokens = self.tokens.saturating_add(tokens);
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counts of characters and tokens stay far below 2^52, where f64 is exact"
)]
pub(crate) const fn to_f64(count: u64) -> f64 {
    count as f64
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/measure/calibration.rs"]
mod tests;
