mod edit;
mod folder;
pub(crate) mod store;
mod view;

pub(crate) use self::edit::{create, insert, replace};
pub(crate) use self::store::open_without_waiting;
pub(crate) use self::view::view;
use crate::text::{CARRIAGE_RETURN, NEW_LINE};

pub(crate) const SNIFFED: u64 = 8 * 1_024;
const NUL: u8 = 0;

pub(crate) fn is_binary(head: &[u8]) -> bool {
    head.contains(&NUL)
}

pub(crate) fn uses_crlf(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .position(|&byte| byte == NEW_LINE)
        .is_some_and(|end| end > 0 && bytes[end - 1] == CARRIAGE_RETURN)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/mod.rs"]
mod tests;
