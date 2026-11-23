use std::io::{self, Write};

use super::Relay;

pub(super) struct Flow<'a, R, W> {
    relay: &'a mut R,
    to: &'a mut W,
    most_held: usize,
    held: Vec<u8>,
    holding: bool,
}

impl<'a, R: Relay, W: Write> Flow<'a, R, W> {
    pub(super) const fn new(relay: &'a mut R, to: &'a mut W, most_held: usize) -> Self {
        Self {
            relay,
            to,
            most_held,
            held: Vec::new(),
            holding: true,
        }
    }

    pub(super) const fn is_holding(&self) -> bool {
        self.holding
    }

    pub(super) fn take(&mut self, piece: &[u8]) -> io::Result<()> {
        self.relay.heard(piece);
        if !self.holding {
            return write(self.to, &self.relay.piece(piece));
        }
        self.held.extend_from_slice(piece);
        if self.held.len() > self.most_held {
            self.let_go()?;
        }
        Ok(())
    }

    pub(super) fn let_go(&mut self) -> io::Result<()> {
        self.holding = false;
        let held = std::mem::take(&mut self.held);
        if held.is_empty() {
            return Ok(());
        }
        write(self.to, &self.relay.piece(&held))
    }

    pub(super) fn pause(&mut self) -> io::Result<()> {
        write(self.to, &self.relay.rest())
    }

    pub(super) fn end(&mut self) -> io::Result<()> {
        if !self.holding {
            return write(self.to, &self.relay.last());
        }
        let shorter = self.relay.whole(&self.held);
        write(self.to, shorter.as_deref().unwrap_or(&self.held))
    }
}

fn write(to: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    to.write_all(bytes)?;
    to.flush()
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/relay/flow.rs"]
mod tests;
