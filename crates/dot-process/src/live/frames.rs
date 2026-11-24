use crate::live::{LENGTH_BYTES, MOST_FRAME, PROCESS_BYTES};

#[derive(Default)]
pub(crate) struct Frames {
    buffer: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct TooLong;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) process: u32,
    pub(crate) command: String,
}

impl Frames {
    pub(crate) fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub(crate) fn next(&mut self) -> Option<Result<Vec<u8>, TooLong>> {
        let length: [u8; LENGTH_BYTES] = self.buffer.get(..LENGTH_BYTES)?.try_into().ok()?;
        let length = usize::try_from(u32::from_be_bytes(length)).unwrap_or(usize::MAX);
        if length > MOST_FRAME {
            return Some(Err(TooLong));
        }
        let end = LENGTH_BYTES.checked_add(length)?;
        if self.buffer.len() < end {
            return None;
        }
        let frame = self.buffer[LENGTH_BYTES..end].to_vec();
        self.buffer.drain(..end);
        Some(Ok(frame))
    }
}

impl Header {
    pub(crate) fn read(frame: &[u8]) -> Option<Self> {
        let (process, command) = frame.split_at_checked(PROCESS_BYTES)?;
        let process: [u8; PROCESS_BYTES] = process.try_into().ok()?;
        Some(Self {
            process: u32::from_be_bytes(process),
            command: String::from_utf8_lossy(command).into_owned(),
        })
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/live/frames.rs"]
mod tests;
