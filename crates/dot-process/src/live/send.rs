use std::io::{self, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use crate::live::frame;
use crate::relay::Relay;

/// One command's output on its way to the [`LiveOutputs`](crate::LiveOutputs) Dot listens to,
/// as the command prints it.
///
/// Sending never waits: a piece that Dot cannot take at once is dropped, so that a command never
/// slows down for being watched. A piece is never sent in part.
#[derive(Debug)]
pub struct LiveOutput {
    stream: UnixStream,
    unsent: Vec<u8>,
}

/// [`Relay`] that also sends each piece of the output to a [`LiveOutput`], as it is read,
/// before `relay` decides what to write.
#[derive(Debug)]
pub struct Watched<R> {
    relay: R,
    live: Option<LiveOutput>,
}

impl LiveOutput {
    /// Connects to the listener at `address` and names the `command` whose output follows.
    ///
    /// # Errors
    ///
    /// Returns an error if nothing listens at `address` or the name cannot be sent.
    pub fn connect(address: &Path, command: &str) -> io::Result<Self> {
        let mut stream = UnixStream::connect(address)?;
        stream.write_all(&frame(command.as_bytes()))?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            unsent: Vec::new(),
        })
    }

    /// Sends `piece`, the last 64 KiB of it at most, unless Dot has not taken the previous one
    /// yet.
    pub fn send(&mut self, piece: &[u8]) {
        if !self.flush() || piece.is_empty() {
            return;
        }
        self.unsent = frame(piece);
        self.flush();
    }

    fn flush(&mut self) -> bool {
        while !self.unsent.is_empty() {
            match self.stream.write(&self.unsent) {
                Ok(0) => return false,
                Ok(written) => {
                    self.unsent.drain(..written);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => return false,
            }
        }
        true
    }
}

impl<R: Relay> Watched<R> {
    /// Wraps `relay`, sending what passes to `live` too when there is one.
    pub const fn new(relay: R, live: Option<LiveOutput>) -> Self {
        Self { relay, live }
    }
}

impl<R: Relay> Relay for Watched<R> {
    fn whole(&mut self, output: &[u8]) -> Option<Vec<u8>> {
        self.relay.whole(output)
    }

    fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
        self.relay.piece(piece)
    }

    fn rest(&mut self) -> Vec<u8> {
        self.relay.rest()
    }

    fn last(&mut self) -> Vec<u8> {
        self.relay.last()
    }

    fn heard(&mut self, piece: &[u8]) {
        if let Some(live) = self.live.as_mut() {
            live.send(piece);
        }
        self.relay.heard(piece);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/live/send.rs"]
mod tests;
