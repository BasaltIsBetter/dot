use std::io::{self, BufReader};
use std::process::{ChildStdout, ExitStatus};
use std::sync::mpsc::{self, Sender};
use std::thread;

use dot_api::Toolbox;
use dot_process::{LiveOutputs, LivePiece, read_line};

use crate::claude_code::EXIT_GRACE;
use crate::claude_code::launch::Running;
use crate::claude_code::listen::{Heard, LONGEST_LINE, Listener};

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

impl<'h, T: Toolbox> Listener<'_, 'h, T> {
    pub(crate) fn listen_to(
        mut self,
        stdout: Option<ChildStdout>,
        live: Option<&LiveOutputs>,
        running: &Running,
    ) -> (io::Result<Heard<'h>>, io::Result<ExitStatus>) {
        thread::scope(|scope| {
            let (sender, receiver) = mpsc::channel();
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
                    let forward = move |piece: LivePiece<'_>| drop(pieces.send(piece_of(piece)));
                    live.serve(&forward);
                });
            }
            drop(sender);

            let heard = self.listen(&receiver);
            drop(self.stdin.take());
            let status = running.finish(EXIT_GRACE);
            live.map(LiveOutputs::stop);
            (heard, status)
        })
    }
}

pub(crate) fn read_lines(stdout: impl io::Read, into: &Sender<Incoming>) {
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
