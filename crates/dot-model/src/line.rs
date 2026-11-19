use std::io::{self, BufRead};

const NEW_LINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Read {
    Line,
    TooLong,
    End,
}

pub(crate) fn read_line(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    longest: usize,
) -> io::Result<Read> {
    line.clear();
    let mut started = false;
    let mut too_long = false;

    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(match (started, too_long) {
                (false, _) => Read::End,
                (true, true) => Read::TooLong,
                (true, false) => finished(line),
            });
        }
        started = true;

        let end = available.iter().position(|&byte| byte == NEW_LINE);
        let piece = &available[..end.unwrap_or(available.len())];
        if !too_long && line.len() + piece.len() > longest {
            too_long = true;
            line.clear();
        }
        if !too_long {
            line.extend_from_slice(piece);
        }
        let used = end.map_or(available.len(), |end| end + 1);
        reader.consume(used);

        if end.is_some() {
            return Ok(if too_long {
                Read::TooLong
            } else {
                finished(line)
            });
        }
    }
}

fn finished(line: &mut Vec<u8>) -> Read {
    if line.last() == Some(&CARRIAGE_RETURN) {
        line.pop();
    }
    Read::Line
}

#[cfg(test)]
#[path = "../../../tests/dot-model/line.rs"]
mod tests;
