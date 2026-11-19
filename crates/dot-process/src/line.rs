use std::io::{self, BufRead};

const NEW_LINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

/// How much of a line [`read_line`] kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// The whole line, without its line break.
    Whole,
    /// Only the start of a line longer than asked; the rest was read past.
    Cut,
}

/// Reads the next line of `reader` into `kept`, `keep` bytes of it at most.
///
/// A line ends with `\n`, `\r\n` or the input, and its line break is never kept. A longer line
/// is read to its end without being held in memory: `kept` receives its first `keep` bytes and
/// the line is [`Line::Cut`]. Returns `None` once nothing is left to read.
///
/// # Errors
///
/// Returns the error of `reader`, except an interrupted read, which is tried again.
///
/// # Examples
///
/// ```
/// use dot_process::{Line, read_line};
///
/// let mut reader = "short\r\nfar too long".as_bytes();
/// let mut kept = Vec::new();
///
/// assert_eq!(read_line(&mut reader, &mut kept, 8).unwrap(), Some(Line::Whole));
/// assert_eq!(kept, b"short");
/// assert_eq!(read_line(&mut reader, &mut kept, 8).unwrap(), Some(Line::Cut));
/// assert_eq!(kept, b"far too ");
/// assert_eq!(read_line(&mut reader, &mut kept, 8).unwrap(), None);
/// ```
pub fn read_line(
    reader: &mut impl BufRead,
    kept: &mut Vec<u8>,
    keep: usize,
) -> io::Result<Option<Line>> {
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
            return Ok(started.then(|| ended(kept, cut)));
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
            return Ok(Some(ended(kept, cut)));
        }
    }
}

/// Counts the lines of `bytes` as an editor shows them.
///
/// A last line without a line break counts; an empty text has no line.
///
/// # Examples
///
/// ```
/// use dot_process::line_count;
///
/// assert_eq!(line_count(b"one\ntwo"), 2);
/// assert_eq!(line_count(b"one\ntwo\n"), 2);
/// assert_eq!(line_count(b""), 0);
/// ```
#[must_use]
pub fn line_count(bytes: &[u8]) -> u64 {
    let breaks = bytes
        .iter()
        .filter(|&&byte| byte == NEW_LINE)
        .fold(0_u64, |count, _| count + 1);
    let unfinished = bytes.last().is_some_and(|&last| last != NEW_LINE);
    breaks + u64::from(unfinished)
}

fn ended(kept: &mut Vec<u8>, cut: bool) -> Line {
    if cut {
        return Line::Cut;
    }
    if kept.last() == Some(&CARRIAGE_RETURN) {
        kept.pop();
    }
    Line::Whole
}

#[cfg(test)]
#[path = "../../../tests/dot-process/line.rs"]
mod tests;
