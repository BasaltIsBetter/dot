mod find;
mod text;

use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use ignore::{Walk, WalkBuilder};

pub(crate) use self::find::find;
pub(crate) use self::text::search;

const SEPARATOR: char = '/';

pub(crate) fn walk(root: &Path) -> Walk {
    WalkBuilder::new(root)
        .standard_filters(true)
        .require_git(false)
        .follow_links(false)
        .sort_by_file_name(Ord::cmp)
        .build()
}

pub(crate) fn glob(pattern: &str) -> Result<GlobMatcher, String> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|error| format!("The glob pattern is not valid: {error}."))
}

pub(crate) fn matches_name_or_path(matcher: &GlobMatcher, pattern: &str, relative: &Path) -> bool {
    if pattern.contains(SEPARATOR) {
        return matcher.is_match(relative);
    }
    relative
        .file_name()
        .is_some_and(|name| matcher.is_match(Path::new(name)))
}

pub(crate) fn count(amount: usize) -> u64 {
    u64::try_from(amount).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/search/mod.rs"]
mod tests;
