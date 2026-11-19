mod edit;
mod folder;
mod store;
mod view;

use std::io::{self, BufRead};

pub(crate) use self::edit::{create, insert, replace};
pub(crate) use self::view::view;

pub(crate) const SNIFFED: u64 = 8 * 1_024;
const NEW_LINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const NUL: u8 = 0;

pub(crate) fn is_binary(head: &[u8]) -> bool {
    head.contains(&NUL)
}

pub(crate) fn line_count(bytes: &[u8]) -> u64 {
    let breaks = bytecount(bytes, NEW_LINE);
    let unfinished = bytes.last().is_some_and(|&last| last != NEW_LINE);
    breaks + u64::from(unfinished)
}

pub(crate) fn uses_crlf(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .position(|&byte| byte == NEW_LINE)
        .is_some_and(|end| end > 0 && bytes[end - 1] == CARRIAGE_RETURN)
}

pub(crate) fn read_line(
    reader: &mut impl BufRead,
    kept: &mut Vec<u8>,
    keep: usize,
) -> io::Result<Option<bool>> {
    kept.clear();
    let mut started = false;
    let mut cut = false;
    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(started.then_some(cut));
        }
        started = true;
        let end = available.iter().position(|&byte| byte == NEW_LINE);
        let piece = &available[..end.unwrap_or(available.len())];
        let room = keep.saturating_sub(kept.len());
        cut |= piece.len() > room;
        kept.extend_from_slice(&piece[..piece.len().min(room)]);
        let used = end.map_or(available.len(), |end| end + 1);
        reader.consume(used);
        if end.is_some() {
            if !cut && kept.last() == Some(&CARRIAGE_RETURN) {
                kept.pop();
            }
            return Ok(Some(cut));
        }
    }
}

fn bytecount(bytes: &[u8], wanted: u8) -> u64 {
    bytes
        .iter()
        .filter(|&&byte| byte == wanted)
        .fold(0, |count, _| count + 1)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/mod.rs"]
mod tests;
