use std::fs::{self, DirBuilder};
use std::io::{self, ErrorKind, Read};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::launch::end_groups;
use crate::live::frames::{Frames, Header};
use crate::live::groups::Groups;
use crate::relay::output::is_readable;

const OWN_FOLDER: u32 = 0o700;
const LOOK: Duration = Duration::from_millis(100);
const FORGET_ENDED_EVERY: Duration = Duration::from_secs(1);
const READ_CHUNK: usize = 16 * 1_024;
const MOST_CONNECTIONS: usize = 16;

/// Where the commands of a program Dot started send what they print as it comes, for Dot to
/// show: a Unix socket in a folder of Dot's own, which only the user can open.
///
/// Each command connects, names itself and the process that leads its group, then sends its
/// output in pieces until it ends. The groups so named are remembered while they run, for
/// [`LiveOutputs::end_commands`]. The socket file is removed when this is dropped.
#[derive(Debug)]
pub struct LiveOutputs {
    listener: UnixListener,
    address: PathBuf,
    stopped: AtomicBool,
    connected: AtomicUsize,
    groups: Groups,
}

/// A piece of what a command printed, as [`LiveOutputs::serve`] hands it over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LivePiece<'a> {
    /// The command's connection, numbered from 0 in the order the commands connected.
    pub connection: u64,
    /// The command, as it named itself.
    pub command: &'a str,
    /// What the command printed, raw, 64 KiB at most; empty once the command is done.
    pub bytes: &'a [u8],
}

impl LiveOutputs {
    /// Listens at `folder/name`, creating `folder` for the user alone if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if `folder` is a link or cannot be made the user's alone, if something
    /// other than a socket is already at `folder/name`, or if the socket cannot be bound (an
    /// address longer than 103 bytes, for one).
    pub fn open(folder: &Path, name: &str) -> io::Result<Self> {
        DirBuilder::new()
            .recursive(true)
            .mode(OWN_FOLDER)
            .create(folder)?;
        let kind = fs::symlink_metadata(folder)?.file_type();
        if !kind.is_dir() {
            return Err(io::Error::new(ErrorKind::InvalidInput, NOT_A_FOLDER));
        }
        fs::set_permissions(folder, fs::Permissions::from_mode(OWN_FOLDER))?;

        let address = folder.join(name);
        match fs::symlink_metadata(&address) {
            Ok(found) if found.file_type().is_socket() => fs::remove_file(&address)?,
            Ok(_) => return Err(io::Error::new(ErrorKind::AlreadyExists, NOT_A_SOCKET)),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            address,
            stopped: AtomicBool::new(false),
            connected: AtomicUsize::new(0),
            groups: Groups::default(),
        })
    }

    /// Returns where the commands connect.
    #[must_use]
    pub fn address(&self) -> &Path {
        &self.address
    }

    /// Hands each piece the commands send to `on_piece`, until [`LiveOutputs::stop`].
    ///
    /// Blocks the calling thread; each command is read on a thread of its own, 16 at most at a
    /// time, so `on_piece` is called from several threads. A command that sends a piece longer
    /// than 64 KiB, or does not name itself first, is no longer read. Once a second, the groups
    /// that no longer run are forgotten, so that their numbers are never signalled after another
    /// program may have taken them.
    pub fn serve(&self, on_piece: &(dyn Fn(LivePiece<'_>) + Sync)) {
        thread::scope(|scope| {
            let mut connection = 0;
            let mut forgotten_at = Instant::now();
            while !self.stopped.load(Ordering::Acquire) {
                if forgotten_at.elapsed() >= FORGET_ENDED_EVERY {
                    self.groups.forget_ended();
                    forgotten_at = Instant::now();
                }
                let Some(stream) = self.next_command() else {
                    continue;
                };
                if self.connected.fetch_add(1, Ordering::AcqRel) >= MOST_CONNECTIONS {
                    self.connected.fetch_sub(1, Ordering::AcqRel);
                    continue;
                }
                let number = connection;
                connection += 1;
                scope.spawn(move || {
                    self.read(stream, number, on_piece);
                    self.connected.fetch_sub(1, Ordering::AcqRel);
                });
            }
        });
    }

    /// Ends [`LiveOutputs::serve`] once the commands it reads have been let go, within a tenth
    /// of a second.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    /// Ends what the commands that named themselves left running: their process groups are
    /// asked to end, given `grace`, then killed.
    ///
    /// A command that runs in the background of its shell (`server &`) outlives the command
    /// Claude Code waits for; this is where it ends with the reply.
    ///
    /// # Errors
    ///
    /// Returns the first error met while signalling; every group is still ended.
    pub fn end_commands(&self, grace: Duration) -> io::Result<()> {
        end_groups(&self.groups.take_all(), grace)
    }

    fn next_command(&self) -> Option<UnixStream> {
        match self.listener.accept() {
            Ok((stream, _)) => match stream.set_nonblocking(false) {
                Ok(()) => Some(stream),
                Err(error) => {
                    tracing::debug!(%error, "a command left before Dot read its live output");
                    None
                }
            },
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                self.wait_for_a_command();
                None
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => None,
            Err(error) => {
                tracing::debug!(%error, "Dot could not take a command's live output");
                thread::sleep(LOOK);
                None
            }
        }
    }

    fn wait_for_a_command(&self) {
        if let Err(error) = is_readable(&self.listener, LOOK) {
            tracing::debug!(%error, "Dot could not wait for a command's live output");
            thread::sleep(LOOK);
        }
    }

    fn read(&self, stream: UnixStream, connection: u64, on_piece: &dyn Fn(LivePiece<'_>)) {
        if let Err(error) = stream.set_read_timeout(Some(LOOK)) {
            tracing::debug!(%error, "a command ended before Dot read its live output");
        }
        self.receive(stream, connection, on_piece);
    }

    fn receive(&self, mut stream: impl Read, connection: u64, on_piece: &dyn Fn(LivePiece<'_>)) {
        let mut frames = Frames::default();
        let mut command: Option<String> = None;
        let mut chunk = vec![0; READ_CHUNK];
        loop {
            if self.stopped.load(Ordering::Acquire) {
                return;
            }
            let read = match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if is_a_pause(&error) => continue,
                Err(_) => break,
            };
            frames.extend(&chunk[..read]);
            while let Some(frame) = frames.next() {
                let Ok(frame) = frame else {
                    return;
                };
                match &command {
                    None => {
                        let Some(header) = Header::read(&frame) else {
                            return;
                        };
                        self.groups.remember(header.process);
                        command = Some(header.command);
                    }
                    Some(command) => on_piece(LivePiece {
                        connection,
                        command,
                        bytes: &frame,
                    }),
                }
            }
        }
        if let Some(command) = &command {
            on_piece(LivePiece {
                connection,
                command,
                bytes: &[],
            });
        }
    }
}

impl Drop for LiveOutputs {
    fn drop(&mut self) {
        drop(fs::remove_file(&self.address));
    }
}

fn is_a_pause(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

const NOT_A_FOLDER: &str = "the folder of live outputs is not a folder";
const NOT_A_SOCKET: &str = "something other than a socket is where live outputs go";

#[cfg(test)]
#[path = "../../../../tests/dot-process/live/listen.rs"]
mod tests;
