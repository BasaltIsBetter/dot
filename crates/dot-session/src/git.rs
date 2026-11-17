use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::readable::is_readable;

const HEAD_PATH: &str = ".git/HEAD";
const LONGEST_HEAD: u64 = 512;
const BRANCH_REFERENCE: &str = "ref: refs/heads/";
const REFERENCE: &str = "ref: ";
const HASH_LENGTHS: [usize; 2] = [40, 64];
const SHORT_HASH: usize = 7;

pub(crate) fn branch(root: &Path) -> Option<String> {
    let head = fs::canonicalize(root.join(HEAD_PATH)).ok()?;
    if !head.starts_with(root) || !fs::metadata(&head).ok()?.is_file() {
        return None;
    }

    let mut contents = String::new();
    File::open(&head)
        .ok()?
        .take(LONGEST_HEAD)
        .read_to_string(&mut contents)
        .ok()?;
    parse_head(&contents)
}

fn parse_head(contents: &str) -> Option<String> {
    let head = contents.lines().next()?.trim();
    let name = if let Some(branch) = head.strip_prefix(BRANCH_REFERENCE) {
        branch
    } else if let Some(reference) = head.strip_prefix(REFERENCE) {
        reference
    } else if is_commit_hash(head) {
        head.get(..SHORT_HASH)?
    } else {
        return None;
    };

    let readable = !name.is_empty() && name.chars().all(is_readable);
    readable.then(|| name.to_owned())
}

fn is_commit_hash(text: &str) -> bool {
    HASH_LENGTHS.contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "../../../tests/dot-session/git.rs"]
pub(crate) mod tests;
