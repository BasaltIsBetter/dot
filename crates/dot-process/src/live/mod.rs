mod frames;
mod groups;
mod listen;
mod send;

pub use self::listen::{LiveOutputs, LivePiece};
pub use self::send::{LiveOutput, Watched};

/// Name of the environment variable that tells the stand-in shell of Claude Code's commands
/// where to send what a command prints as it comes: the address of a [`LiveOutputs`].
pub const LIVE_OUTPUT_VARIABLE: &str = "DOT_LIVE_OUTPUT";

const LENGTH_BYTES: usize = 4;
const PROCESS_BYTES: usize = 4;
const MOST_FRAME: usize = 64 * 1_024;

fn frame(bytes: &[u8]) -> Vec<u8> {
    let kept = &bytes[bytes.len().saturating_sub(MOST_FRAME)..];
    let length = u32::try_from(kept.len()).unwrap_or(u32::MAX);
    let mut framed = Vec::with_capacity(LENGTH_BYTES + kept.len());
    framed.extend_from_slice(&length.to_be_bytes());
    framed.extend_from_slice(kept);
    framed
}

fn named(process: u32, command: &str) -> Vec<u8> {
    let name = &command.as_bytes()[..command.len().min(MOST_FRAME - PROCESS_BYTES)];
    let mut header = Vec::with_capacity(PROCESS_BYTES + name.len());
    header.extend_from_slice(&process.to_be_bytes());
    header.extend_from_slice(name);
    frame(&header)
}

#[cfg(test)]
#[path = "../../../../tests/dot-process/live/mod.rs"]
mod tests;
