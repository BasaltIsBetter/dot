use std::collections::VecDeque;
use std::path::PathBuf;

use crate::text::NEW_LINE;

const HEAD: usize = 15_000;
const TAIL: usize = 15_000;
const MOST_FRESH: usize = 64 * 1_024;

#[derive(Debug, Default)]
pub(crate) struct Capture {
    head: Vec<u8>,
    tail: VecDeque<u8>,
    cut: u64,
    fresh: Vec<u8>,
    abandoned: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Heard {
    pub(crate) text: String,
    pub(crate) folder: Option<PathBuf>,
}

impl Capture {
    pub(crate) fn abandoned() -> Self {
        Self {
            abandoned: true,
            ..Self::default()
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) {
        if self.abandoned {
            return;
        }
        self.fresh.extend_from_slice(bytes);
        let stale = self.fresh.len().saturating_sub(MOST_FRESH);
        self.fresh.drain(..stale);

        let room = HEAD - self.head.len();
        let (head, rest) = bytes.split_at(room.min(bytes.len()));
        self.head.extend_from_slice(head);
        self.tail.extend(rest);
        let excess = self.tail.len().saturating_sub(TAIL);
        self.tail.drain(..excess);
        self.cut += u64::try_from(excess).unwrap_or(u64::MAX);
    }

    pub(crate) fn take_fresh(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.fresh)
    }

    pub(crate) fn heard(self, sentinel: &str) -> Heard {
        let cut = self.cut;
        let head_length = self.head.len();
        let mut bytes = self.head;
        bytes.extend(self.tail);

        let marker = [&[NEW_LINE], sentinel.as_bytes()].concat();
        let found = bytes
            .windows(marker.len())
            .rposition(|window| window == marker.as_slice());
        let folder = found.map(|at| {
            let after = &bytes[at + marker.len()..];
            let after = after.strip_suffix(&[NEW_LINE]).unwrap_or(after);
            PathBuf::from(String::from_utf8_lossy(after).into_owned())
        });
        if let Some(at) = found {
            bytes.truncate(at);
        }

        let text = if cut == 0 {
            String::from_utf8_lossy(&bytes).into_owned()
        } else {
            let (head, tail) = bytes.split_at(head_length.min(bytes.len()));
            format!(
                "{}\n[… {cut} bytes of output cut here …]\n{}",
                String::from_utf8_lossy(head),
                String::from_utf8_lossy(tail)
            )
        };
        Heard { text, folder }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/shell/output.rs"]
mod tests;
