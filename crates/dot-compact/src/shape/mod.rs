mod json;
mod matches;
mod paths;
mod prefix;
mod tree;

pub(crate) use self::json::{is_document, minify, minify_after_lines};
pub(crate) use self::matches::{group as group_matches, name_folder};
pub(crate) use self::paths::group as group_paths;
pub(crate) use self::prefix::{name_neighbours, note as folder_note, shared_folder};
pub(crate) use self::tree::flatten as flatten_tree;

const HUNK_START: &str = "@@ -";
const HUNK_END: &str = " @@";
const GIT_DIFF: &str = "diff --git ";

pub(crate) fn is_diff(text: &str) -> bool {
    text.lines().any(|line| {
        line.starts_with(GIT_DIFF) || (line.starts_with(HUNK_START) && line.contains(HUNK_END))
    })
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/shape/mod.rs"]
mod tests;
