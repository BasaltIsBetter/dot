use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::machine::Places;

const FOLDER: &str = "dot";
const DATA_VARIABLE: &str = "XDG_DATA_HOME";
const DATA: &str = ".local/share";
const STATE_VARIABLE: &str = "XDG_STATE_HOME";
const STATE: &str = ".local/state";
const LANGUAGES_VARIABLE: &str = "LANGUAGE";
const LANGUAGE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];
const LANGUAGE_SEPARATOR: char = ':';
const ENCODING_SEPARATORS: [char; 2] = ['.', '@'];
const NO_LANGUAGE: [&str; 2] = ["C", "POSIX"];

/// The folders of the XDG Base Directory specification: the variables that are set and absolute
/// win over the places they stand for under `home`.
pub(crate) fn places(home: &Path) -> Places {
    places_among(home, |variable| env::var_os(variable))
}

/// The languages of the locale, the first one preferred.
pub(crate) fn preferred_languages() -> Vec<String> {
    languages_among(|variable| env::var(variable).ok())
}

fn places_among(home: &Path, variable: impl Fn(&str) -> Option<OsString>) -> Places {
    let base = |name: &str, usual: &str| {
        variable(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(usual))
    };
    Places {
        workspace: base(DATA_VARIABLE, DATA).join(FOLDER),
        logs: base(STATE_VARIABLE, STATE).join(FOLDER),
    }
}

fn languages_among(variable: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let locale = LANGUAGE_VARIABLES
        .into_iter()
        .filter_map(&variable)
        .find(|value| !value.is_empty());
    let Some(locale) = locale.as_deref().and_then(language) else {
        return Vec::new();
    };
    let listed = variable(LANGUAGES_VARIABLE).unwrap_or_default();
    listed
        .split(LANGUAGE_SEPARATOR)
        .filter_map(language)
        .chain([locale])
        .collect()
}

fn language(locale: &str) -> Option<String> {
    let language = locale.split(ENCODING_SEPARATORS).next()?;
    let names_one = !language.is_empty() && !NO_LANGUAGE.contains(&language);
    names_one.then(|| language.to_owned())
}

#[cfg(test)]
#[path = "../../../../tests/dot/machine/linux.rs"]
mod tests;
