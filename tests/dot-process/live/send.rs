use std::fs;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;

struct Socket {
    folder: PathBuf,
    listener: UnixListener,
}

impl Socket {
    fn new(name: &str) -> Self {
        let folder = std::env::temp_dir().join(format!("dot-send-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&folder));
        fs::create_dir_all(&folder).unwrap();
        let listener = UnixListener::bind(folder.join("s")).unwrap();
        Self { folder, listener }
    }

    fn address(&self) -> PathBuf {
        self.folder.join("s")
    }

    fn received(&self) -> Vec<u8> {
        let (mut stream, _) = self.listener.accept().unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 1_024];
        while let Ok(read) = stream.read(&mut chunk) {
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
        }
        bytes
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.folder));
    }
}

struct Echo {
    heard: Vec<u8>,
}

impl Relay for Echo {
    fn whole(&mut self, _output: &[u8]) -> Option<Vec<u8>> {
        Some(b"whole".to_vec())
    }

    fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
        piece.to_ascii_uppercase()
    }

    fn rest(&mut self) -> Vec<u8> {
        b"rest".to_vec()
    }

    fn last(&mut self) -> Vec<u8> {
        b"last".to_vec()
    }

    fn heard(&mut self, piece: &[u8]) {
        self.heard.extend_from_slice(piece);
    }
}

#[test]
fn names_the_command_then_sends_each_piece_framed() {
    let socket = Socket::new("pieces");
    let mut live = LiveOutput::connect(&socket.address(), "ls").unwrap();

    live.send(b"a\n");
    live.send(b"");
    live.send(b"b\n");
    drop(live);

    let mut expected = frame(b"ls");
    expected.extend(frame(b"a\n"));
    expected.extend(frame(b"b\n"));
    assert_eq!(socket.received(), expected);
}

#[test]
fn cannot_connect_where_nothing_listens() {
    let folder = std::env::temp_dir().join("dot-send-nobody-listens");

    assert!(LiveOutput::connect(&folder.join("s"), "ls").is_err());
}

#[test]
fn never_waits_for_a_listener_that_does_not_read() {
    let socket = Socket::new("full");
    let mut live = LiveOutput::connect(&socket.address(), "yes").unwrap();
    let piece = vec![b'y'; 64 * 1_024];
    let started = Instant::now();

    for _ in 0..200 {
        live.send(&piece);
    }

    assert!(started.elapsed() < Duration::from_secs(2));
    drop(live);
    let received = socket.received();
    assert!(received.len() < 200 * piece.len());
    let mut frames = 0;
    let mut rest = &received[frame(b"yes").len()..];
    while rest.len() >= 4 + piece.len() {
        let length = usize::try_from(u32::from_be_bytes(rest[..4].try_into().unwrap())).unwrap();
        assert_eq!(length, piece.len(), "frames stay whole and in step");
        rest = &rest[4 + length..];
        frames += 1;
    }
    assert!(
        rest.len() < 4 + piece.len(),
        "only the frame cut by the close is left"
    );
    assert!(frames < 200);
    assert!(received.starts_with(&frame(b"yes")));
}

#[test]
fn a_watched_relay_sends_what_it_hears_and_lets_its_relay_decide_the_rest() {
    let socket = Socket::new("watched");
    let live = LiveOutput::connect(&socket.address(), "ls").unwrap();
    let mut watched = Watched::new(Echo { heard: Vec::new() }, Some(live));

    watched.heard(b"ab");
    let piece = watched.piece(b"ab");
    let whole = watched.whole(b"ab");
    let rest = watched.rest();
    let last = watched.last();
    let echo = watched.relay.heard.clone();
    drop(watched);

    assert_eq!(
        (piece, whole, rest, last),
        (
            b"AB".to_vec(),
            Some(b"whole".to_vec()),
            b"rest".to_vec(),
            b"last".to_vec()
        )
    );
    assert_eq!(echo, b"ab");
    let mut expected = frame(b"ls");
    expected.extend(frame(b"ab"));
    assert_eq!(socket.received(), expected);
}

#[test]
fn a_watched_relay_without_a_live_output_only_hears() {
    let mut watched = Watched::new(Echo { heard: Vec::new() }, None);

    watched.heard(b"ab");

    assert_eq!(watched.relay.heard, b"ab");
}
