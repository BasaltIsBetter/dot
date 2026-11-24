use std::io::{self, PipeReader, Read};
use std::os::fd::AsFd;
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::retry_on_intr;

pub(crate) enum Heard<'a> {
    Piece(&'a [u8]),
    Nothing,
    Closed,
}

pub(crate) struct Output {
    pipe: PipeReader,
    piece: Vec<u8>,
}

impl Output {
    pub(crate) fn new(pipe: PipeReader, largest_piece: usize) -> Self {
        Self {
            pipe,
            piece: vec![0; largest_piece],
        }
    }

    pub(crate) fn hear(&mut self, wait: Duration) -> io::Result<Heard<'_>> {
        if !is_readable(&self.pipe, wait)? {
            return Ok(Heard::Nothing);
        }
        let read = read_piece(&mut self.pipe, &mut self.piece)?;
        if read == 0 {
            return Ok(Heard::Closed);
        }
        Ok(Heard::Piece(&self.piece[..read]))
    }

    pub(super) fn into_pipe(self) -> PipeReader {
        self.pipe
    }
}

pub(crate) fn is_readable(source: impl AsFd, wait: Duration) -> io::Result<bool> {
    let wait = Timespec::try_from(wait).map_err(io::Error::other)?;
    let mut watched = [PollFd::new(&source, PollFlags::IN)];
    let ready = retry_on_intr(|| poll(&mut watched, Some(&wait)))?;
    Ok(ready > 0)
}

fn read_piece(source: &mut impl Read, piece: &mut [u8]) -> io::Result<usize> {
    loop {
        match source.read(piece) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            read => return read,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/relay/output.rs"]
mod tests;
