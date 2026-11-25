use std::path::Path;

use objc2_foundation::NSLocale;

use crate::machine::Places;

const WORKSPACE: &str = "Library/Application Support/Dot";
const LOGS: &str = "Library/Logs/Dot";

pub(crate) fn places(home: &Path) -> Places {
    Places {
        workspace: home.join(WORKSPACE),
        logs: home.join(LOGS),
    }
}

pub(crate) fn preferred_languages() -> Vec<String> {
    NSLocale::preferredLanguages()
        .iter()
        .map(|tag| tag.to_string())
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/dot/machine/macos.rs"]
mod tests;
