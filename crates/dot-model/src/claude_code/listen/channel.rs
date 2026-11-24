use std::io::{self, BufReader};
use std::process::{ChildStdout, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;

use dot_api::Toolbox;
use dot_process::{LiveOutputs, LivePiece, read_line};

use crate::claude_code::EXIT_GRACE;
use crate::claude_code::launch::{CANCEL_GRACE, Running};
use crate::claude_code::listen::{Heard, LONGEST_LINE, Listener};

pub(crate) const QUEUED: usize = 64;

#[derive(Debug)]
pub(crate) enum Incoming {
    Line(Vec<u8>),
    Piece {
        connection: u64,
        command: String,
        bytes: Vec<u8>,
    },
    Ended(io::Result<()>),
}

struct Ending<'a> {
    finished: &'a AtomicBool,
    live: Option<&'a LiveOutputs>,
}

impl<'h, T: Toolbox> Listener<'_, 'h, T> {
    pub(crate) fn listen_to(
        mut self,
        stdout: Option<ChildStdout>,
        live: Option<&LiveOutputs>,
        running: &Running,
    ) -> (io::Result<Heard<'h>>, io::Result<ExitStatus>) {
        let cancellation = self.hands.cancellation;
        let finished = AtomicBool::new(false);
        thread::scope(|scope| {
            let ending = Ending {
                finished: &finished,
                live,
            };
            let (sender, receiver) = mpsc::sync_channel(QUEUED);
            match stdout {
                Some(stdout) => {
                    let lines = sender.clone();
                    let reader = thread::spawn(move || read_lines(stdout, &lines));
                    drop(reader);
                }
                None => drop(sender.send(Incoming::Ended(Ok(())))),
            }
            if let Some(live) = live {
                let pieces = sender.clone();
                scope.spawn(move || {
                    let forward =
                        move |piece: LivePiece<'_>| drop(pieces.try_send(piece_of(piece)));
                    live.serve(&forward);
                });
            }
            scope.spawn(|| running.end_when_cancelled(cancellation, &finished, CANCEL_GRACE));
            drop(sender);

            let heard = self.listen(&receiver);
            drop(self.stdin.take());
            let grace = if cancellation.is_cancelled() {
                CANCEL_GRACE
            } else {
                EXIT_GRACE
            };
            let status = running.finish(grace);
            drop(ending);
            (heard, status)
        })
    }
}

impl Drop for Ending<'_> {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Release);
        let Some(live) = self.live else {
            return;
        };
        if let Err(error) = live.end_commands(dot_process::GRACE) {
            tracing::warn!(%error, "Dot could not end what Claude's commands left running");
        }
        live.stop();
    }
}

pub(crate) fn read_lines(stdout: impl io::Read, into: &SyncSender<Incoming>) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        let arrived = match read_line(&mut reader, &mut line, LONGEST_LINE) {
            Ok(Some(dot_process::Line::Cut)) => continue,
            Ok(Some(dot_process::Line::Whole)) => Incoming::Line(std::mem::take(&mut line)),
            Ok(None) => Incoming::Ended(Ok(())),
            Err(error) => Incoming::Ended(Err(error)),
        };
        let last = matches!(arrived, Incoming::Ended(_));
        if into.send(arrived).is_err() || last {
            return;
        }
    }
}

fn piece_of(piece: LivePiece<'_>) -> Incoming {
    Incoming::Piece {
        connection: piece.connection,
        command: piece.command.to_owned(),
        bytes: piece.bytes.to_vec(),
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/listen/channel.rs"]
mod tests;
