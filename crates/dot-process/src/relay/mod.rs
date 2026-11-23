mod flow;
pub(crate) mod output;
mod timing;

use std::io::{self, PipeReader, Write};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

use self::flow::Flow;
use self::output::{Heard, Output};
use self::timing::Timing;

const PIECE: usize = 64 * 1_024;
const LOOK: Duration = Duration::from_millis(10);
const MOST_PIECES_AFTER_THE_END: usize = 16;

/// What decides how the output of a relayed program is passed on.
pub trait Relay {
    /// Returns what to write instead of `output`, the whole output of a program that ended
    /// while it was still held; `None` writes `output` as it is.
    fn whole(&mut self, output: &[u8]) -> Option<Vec<u8>>;

    /// Returns what to write for the next `piece` of the output of a program that still runs.
    fn piece(&mut self, piece: &[u8]) -> Vec<u8>;

    /// Returns what [`Relay::piece`] held back, when the output pauses or has gone on without
    /// a pause for [`Pace::wait`].
    fn rest(&mut self) -> Vec<u8>;

    /// Returns what ends the output of a program that was passed on piece by piece: what
    /// [`Relay::rest`] returns, unless the relay has more to say once nothing follows.
    fn last(&mut self) -> Vec<u8> {
        self.rest()
    }

    /// Sees each piece of the output as it is read, before anything decides what to write;
    /// nothing by default.
    fn heard(&mut self, _piece: &[u8]) {}
}

/// Where the errors a relayed program writes go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errors {
    /// Into the relayed output, in the order the program wrote them.
    Joined,
    /// Wherever the command already sends them, untouched.
    Apart,
}

/// How long and how much of an output a relay holds before it passes it on as it comes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    /// Longest time the output of a program that still runs is held whole.
    pub hold: Duration,
    /// Silence after which what is held back from a passing output is written.
    pub pause: Duration,
    /// Longest time what is held back from a passing output waits for such a silence.
    pub wait: Duration,
    /// Most bytes held whole.
    pub held: usize,
}

/// How a relayed program ended.
#[derive(Debug)]
pub struct Relayed {
    /// How the program itself ended.
    pub status: ExitStatus,
    /// The output of the program, when something it left running still holds it open. What is
    /// written there from now on is the caller's to read; dropping this breaks the pipe of
    /// whoever still writes.
    pub left_open: Option<PipeReader>,
}

/// Runs `command` and writes its output to `to` through `relay`, then returns how it ended.
///
/// The output is held until the program ends, so that `relay` sees it whole. A program that
/// outlasts `pace.hold`, or writes more than `pace.held` bytes, has its output passed on piece
/// by piece from then on, so that whoever watches it sees it advance: what `relay` holds back
/// is written after `pace.pause` of silence, and every `pace.wait` when there is no silence.
///
/// Blocks until the program has ended and everything it wrote is written, not until what it
/// left running in the background ends: the output such a program still holds open is handed
/// back in [`Relayed::left_open`], after what came within a hundredth of a second of the end.
///
/// # Errors
///
/// Returns an error if the program cannot start, if its output cannot be read or if `to`
/// cannot be written. The program is then stopped; what it started itself is not.
///
/// # Examples
///
/// ```
/// use std::process::Command;
/// use std::time::Duration;
///
/// use dot_process::{Errors, Pace, Relay, relay};
///
/// struct Shouting;
///
/// impl Relay for Shouting {
///     fn whole(&mut self, output: &[u8]) -> Option<Vec<u8>> {
///         Some(output.to_ascii_uppercase())
///     }
///     fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
///         piece.to_vec()
///     }
///     fn rest(&mut self) -> Vec<u8> {
///         Vec::new()
///     }
/// }
///
/// let mut command = Command::new("/bin/echo");
/// command.arg("hello");
/// let pace = Pace {
///     hold: Duration::from_secs(5),
///     pause: Duration::from_secs(1),
///     wait: Duration::from_secs(30),
///     held: 1_024,
/// };
/// let mut written = Vec::new();
///
/// let relayed = relay(command, Errors::Joined, pace, &mut Shouting, &mut written).unwrap();
///
/// assert!(relayed.status.success());
/// assert!(relayed.left_open.is_none());
/// assert_eq!(written, b"HELLO\n");
/// ```
pub fn relay(
    mut command: Command,
    errors: Errors,
    pace: Pace,
    relay: &mut impl Relay,
    to: &mut impl Write,
) -> io::Result<Relayed> {
    let (pipe, writer) = io::pipe()?;
    if errors == Errors::Joined {
        command.stderr(writer.try_clone()?);
    }
    command.stdout(writer);
    let mut child = command.spawn()?;
    drop(command);

    let mut output = Output::new(pipe, PIECE);
    let mut flow = Flow::new(relay, to, pace.held);
    match pass(&mut output, &mut child, pace, &mut flow) {
        Ok(Some(status)) => Ok(Relayed {
            status,
            left_open: Some(output.into_pipe()),
        }),
        Ok(None) => Ok(Relayed {
            status: child.wait()?,
            left_open: None,
        }),
        Err(error) => {
            child.kill()?;
            child.wait()?;
            Err(error)
        }
    }
}

fn pass<R: Relay, W: Write>(
    output: &mut Output,
    child: &mut Child,
    pace: Pace,
    flow: &mut Flow<'_, R, W>,
) -> io::Result<Option<ExitStatus>> {
    let mut timing = Timing::starting(pace, Instant::now());
    loop {
        let due = timing.settle(Instant::now(), flow)?;
        match output.hear(LOOK.min(due))? {
            Heard::Piece(piece) => {
                flow.take(piece)?;
                timing.heard(Instant::now());
            }
            Heard::Nothing => {}
            Heard::Closed => {
                flow.end()?;
                return Ok(None);
            }
        }
        if let Some(status) = child.try_wait()? {
            let closed = drain(output, flow)?;
            flow.end()?;
            return Ok((!closed).then_some(status));
        }
    }
}

fn drain<R: Relay, W: Write>(output: &mut Output, flow: &mut Flow<'_, R, W>) -> io::Result<bool> {
    for _ in 0..MOST_PIECES_AFTER_THE_END {
        match output.hear(LOOK)? {
            Heard::Piece(piece) => flow.take(piece)?,
            Heard::Nothing => return Ok(false),
            Heard::Closed => return Ok(true),
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/relay/mod.rs"]
mod tests;
