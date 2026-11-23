//! Hot paths of Claude's replies, opened to the benches of `benches/` only.
//!
//! Compiled with the `bench` feature, never in the app.

use std::io::{BufReader, Cursor};
use std::sync::mpsc;
use std::thread;

use dot_api::{Cancellation, Effort, Model};

use crate::claude_code::{Incoming, parse, read_lines};
use crate::messages::{self, BenchTurn as Turn};

const SYSTEM: &str = "You are Claude, working in a folder of the user's Mac.";

/// Returns an SSE stream of Claude's reply carrying `length` bytes of text, in pieces of about
/// 20 bytes, as Anthropic streams it.
#[must_use]
pub fn reply_stream(length: usize) -> Vec<u8> {
    let mut stream = String::from(concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-opus-5-5\",",
        "\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,",
        "\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
    ));
    for _ in 0..length / 20 {
        stream.push_str(concat!(
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,",
            "\"delta\":{\"type\":\"text_delta\",\"text\":\"a few words of text \"}}\n\n",
        ));
    }
    stream.push_str(concat!(
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},",
        "\"usage\":{\"output_tokens\":5}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    ));
    stream.into_bytes()
}

/// Reads `stream` as Dot reads a reply and returns how many pieces it handed over.
#[must_use]
pub fn read_reply(stream: &[u8]) -> usize {
    let mut turn = Turn::replayable(None);
    let mut pieces = 0;
    let read = messages::read_stream(
        Cursor::new(stream),
        &mut turn,
        &mut |_| pieces += 1,
        &Cancellation::new(),
    );
    debug_assert!(read.is_ok(), "the bench stream is a whole reply");
    pieces
}

/// Prepares what is sent before a conversation of `history` bytes, as each turn does, and
/// returns how many bytes the request carries in all.
#[must_use]
pub fn prepare_request(history: &[u8]) -> usize {
    let head = messages::head(Model::OPUS_5_5.id, Effort::High, SYSTEM);
    head.map_or(0, |head| head.len() + history.len())
}

/// Returns what Claude Code prints of a reply carrying `length` bytes of text, one JSON line per
/// piece of about 20 bytes, as it streams it.
#[must_use]
pub fn claude_code_output(length: usize) -> Vec<u8> {
    let piece = concat!(
        "{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",",
        "\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"a few words of text \"}}}\n",
    );
    piece.repeat(length / 20).into_bytes()
}

/// Reads `output` as Dot reads Claude Code's: a thread reads each line and hands it over a
/// channel, the listener decodes it; returns how many lines were decoded.
#[must_use]
pub fn read_claude_code(output: &'static [u8]) -> usize {
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || read_lines(Cursor::new(output), &sender));
    let decoded = receiver
        .iter()
        .take_while(|arrived| !matches!(arrived, Incoming::Ended(_)))
        .filter(|arrived| matches!(arrived, Incoming::Line(line) if parse(line).is_ok()))
        .count();
    debug_assert!(
        reader.join().is_ok(),
        "the reader of the bench never panics"
    );
    decoded
}

/// Reads `output` line by line on the calling thread, one buffer for every line: how Dot read
/// Claude Code's output before the live output of commands.
#[must_use]
pub fn read_claude_code_directly(output: &[u8]) -> usize {
    let mut reader = BufReader::new(output);
    let mut line = Vec::new();
    let mut decoded = 0;
    while let Ok(Some(read)) = dot_process::read_line(&mut reader, &mut line, usize::MAX) {
        if matches!(read, dot_process::Line::Whole) && parse(&line).is_ok() {
            decoded += 1;
        }
    }
    decoded
}
