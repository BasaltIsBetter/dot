use std::collections::VecDeque;

use dot_api::{Compactor, LIVE_OUTPUT_LINES};

const MOST_LINE_BYTES: usize = 1_024;
const LINE_BREAK: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

#[derive(Debug, Default)]
pub(crate) struct LiveTail {
    finished: VecDeque<Vec<u8>>,
    current: Vec<u8>,
    returned: bool,
}

impl LiveTail {
    pub(crate) fn push(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.returned {
                self.returned = false;
                if byte != LINE_BREAK {
                    self.current.clear();
                }
            }
            match byte {
                LINE_BREAK => self.finish_line(),
                CARRIAGE_RETURN => self.returned = true,
                _ if self.current.len() < MOST_LINE_BYTES => self.current.push(byte),
                _ => {}
            }
        }
    }

    pub(crate) fn lines(&self, compactor: &dyn Compactor) -> Vec<String> {
        let growing = (!self.current.is_empty()).then_some(&self.current);
        let kept = LIVE_OUTPUT_LINES - usize::from(growing.is_some());
        let skipped = self.finished.len().saturating_sub(kept);
        self.finished
            .iter()
            .skip(skipped)
            .chain(growing)
            .map(|line| {
                compactor
                    .readable(&String::from_utf8_lossy(line))
                    .into_owned()
            })
            .collect()
    }

    fn finish_line(&mut self) {
        let line = std::mem::take(&mut self.current);
        self.finished.push_back(line);
        if self.finished.len() > LIVE_OUTPUT_LINES {
            self.finished.pop_front();
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/live/mod.rs"]
mod tests;
