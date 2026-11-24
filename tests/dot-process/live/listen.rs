use std::io::Write;
use std::os::unix::fs::symlink;
use std::os::unix::process::ExitStatusExt;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Instant;

use super::*;
use crate::launch::{GRACE, command, group_is_alive, stop};
use crate::live::{frame, named};

const NOT_A_LEADER: u32 = 0;

struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("dot-live-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&path));
        Self(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

type Heard = Vec<(u64, String, Vec<u8>)>;

fn serving(outputs: &LiveOutputs, heard: &Mutex<Heard>, until: impl FnOnce()) {
    let record = |piece: LivePiece<'_>| {
        heard.lock().unwrap().push((
            piece.connection,
            piece.command.to_owned(),
            piece.bytes.to_vec(),
        ));
    };
    thread::scope(|scope| {
        scope.spawn(|| outputs.serve(&record));
        until();
        outputs.stop();
    });
}

fn wait_for(heard: &Mutex<Heard>, count: usize) {
    let started = Instant::now();
    while heard.lock().unwrap().len() < count {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "pieces never came"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn creates_its_folder_for_the_user_alone_and_listens_there() {
    let folder = Folder::new("open");

    let outputs = LiveOutputs::open(&folder.0.join("live"), "a").unwrap();

    let mode = fs::metadata(folder.0.join("live"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
    assert_eq!(outputs.address(), folder.0.join("live").join("a"));
    assert!(
        fs::symlink_metadata(outputs.address())
            .unwrap()
            .file_type()
            .is_socket()
    );
}

#[test]
fn removes_its_socket_when_dropped() {
    let folder = Folder::new("drop");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let address = outputs.address().to_path_buf();

    drop(outputs);

    assert!(!address.exists());
}

#[test]
fn takes_the_place_of_a_socket_left_behind_but_of_nothing_else() {
    let folder = Folder::new("stale");
    let stale = UnixListener::bind({
        fs::create_dir_all(&folder.0).unwrap();
        folder.0.join("a")
    })
    .unwrap();
    drop(stale);
    fs::write(folder.0.join("b"), "a file").unwrap();

    let replaced = LiveOutputs::open(&folder.0, "a");
    let refused = LiveOutputs::open(&folder.0, "b");

    assert!(replaced.is_ok());
    assert_eq!(refused.unwrap_err().kind(), ErrorKind::AlreadyExists);
    assert_eq!(fs::read_to_string(folder.0.join("b")).unwrap(), "a file");
}

#[test]
fn returns_what_stopped_it_from_looking_at_the_place_of_its_socket() {
    let folder = Folder::new("unnamable");
    let longer_than_a_file_name = "a".repeat(300);

    let opened = LiveOutputs::open(&folder.0, &longer_than_a_file_name);

    assert_eq!(opened.unwrap_err().kind(), ErrorKind::InvalidFilename);
}

#[test]
fn refuses_a_folder_that_is_a_link() {
    let folder = Folder::new("link");
    fs::create_dir_all(folder.0.join("real")).unwrap();
    symlink(folder.0.join("real"), folder.0.join("linked")).unwrap();

    let opened = LiveOutputs::open(&folder.0.join("linked"), "a");

    assert_eq!(opened.unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn hands_over_each_piece_with_its_command_then_the_end() {
    let folder = Folder::new("serve");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());

    serving(&outputs, &heard, || {
        let mut stream = UnixStream::connect(outputs.address()).unwrap();
        stream
            .write_all(&named(NOT_A_LEADER, "cargo test"))
            .unwrap();
        stream.write_all(&frame(b"Compiling dot\n")).unwrap();
        let second = frame(b"Finished\n");
        stream.write_all(&second[..3]).unwrap();
        thread::sleep(Duration::from_millis(20));
        stream.write_all(&second[3..]).unwrap();
        drop(stream);
        wait_for(&heard, 3);
    });

    let heard = heard.into_inner().unwrap();
    assert_eq!(
        heard,
        [
            (0, "cargo test".to_owned(), b"Compiling dot\n".to_vec()),
            (0, "cargo test".to_owned(), b"Finished\n".to_vec()),
            (0, "cargo test".to_owned(), Vec::new()),
        ]
    );
}

#[test]
fn numbers_each_command_in_the_order_it_connected() {
    let folder = Folder::new("numbers");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());

    serving(&outputs, &heard, || {
        for command in ["first", "second"] {
            let mut stream = UnixStream::connect(outputs.address()).unwrap();
            stream.write_all(&named(NOT_A_LEADER, command)).unwrap();
            stream.write_all(&frame(b"x")).unwrap();
            wait_for(&heard, if command == "first" { 1 } else { 3 });
            drop(stream);
        }
        wait_for(&heard, 4);
    });

    let commands: Vec<(u64, String)> = heard
        .into_inner()
        .unwrap()
        .into_iter()
        .filter(|(_, _, bytes)| !bytes.is_empty())
        .map(|(connection, command, _)| (connection, command))
        .collect();
    assert_eq!(
        commands,
        [(0, "first".to_owned()), (1, "second".to_owned())]
    );
}

#[test]
fn stops_reading_a_command_that_sends_a_piece_too_long() {
    let folder = Folder::new("long");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());

    serving(&outputs, &heard, || {
        let mut stream = UnixStream::connect(outputs.address()).unwrap();
        stream.write_all(&named(NOT_A_LEADER, "cat")).unwrap();
        stream.write_all(&65_537_u32.to_be_bytes()).unwrap();
        stream.write_all(&frame(b"never seen")).unwrap();
        thread::sleep(Duration::from_millis(200));
    });

    assert_eq!(heard.into_inner().unwrap(), Heard::new());
}

#[test]
fn stops_within_a_moment_even_while_a_command_is_still_connected() {
    let folder = Folder::new("stop");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();

    thread::scope(|scope| {
        scope.spawn(|| {
            outputs.serve(&|_| {});
            sender.send(()).unwrap();
        });
        let mut stream = UnixStream::connect(outputs.address()).unwrap();
        stream.write_all(&named(NOT_A_LEADER, "sleep 30")).unwrap();
        thread::sleep(Duration::from_millis(50));
        outputs.stop();
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(stream);
    });

    assert!(started.elapsed() < Duration::from_secs(1));
}

struct Failing {
    given: Vec<io::Result<Vec<u8>>>,
}

impl Read for Failing {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.given.is_empty() {
            return Err(ErrorKind::WouldBlock.into());
        }
        let bytes = self.given.remove(0)?;
        into[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

#[test]
fn ends_a_command_whose_connection_fails_and_waits_through_its_pauses() {
    let folder = Folder::new("failing");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());
    let mut first = named(NOT_A_LEADER, "ls");
    first.extend(frame(b"a"));
    let stream = Failing {
        given: vec![
            Ok(first),
            Err(ErrorKind::WouldBlock.into()),
            Err(ErrorKind::TimedOut.into()),
            Err(ErrorKind::Interrupted.into()),
            Ok(frame(b"b")),
            Err(ErrorKind::ConnectionReset.into()),
            Ok(frame(b"never")),
        ],
    };

    outputs.receive(stream, 4, &|piece| {
        heard.lock().unwrap().push((
            piece.connection,
            piece.command.to_owned(),
            piece.bytes.to_vec(),
        ));
    });

    assert_eq!(
        heard.into_inner().unwrap(),
        [
            (4, "ls".to_owned(), b"a".to_vec()),
            (4, "ls".to_owned(), b"b".to_vec()),
            (4, "ls".to_owned(), Vec::new()),
        ]
    );
}

#[test]
fn waits_for_a_command_that_falls_silent_for_a_while() {
    let folder = Folder::new("silent");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());

    serving(&outputs, &heard, || {
        let mut stream = UnixStream::connect(outputs.address()).unwrap();
        stream.write_all(&named(NOT_A_LEADER, "sleep")).unwrap();
        stream.write_all(&frame(b"before")).unwrap();
        thread::sleep(Duration::from_millis(350));
        stream.write_all(&frame(b"after")).unwrap();
        drop(stream);
        wait_for(&heard, 3);
    });

    let pieces: Vec<Vec<u8>> = heard
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|(_, _, bytes)| bytes)
        .collect();
    assert_eq!(pieces, [b"before".to_vec(), b"after".to_vec(), Vec::new()]);
}

fn leader() -> std::process::Child {
    command("/bin/sleep").arg("30").spawn().unwrap()
}

fn named_by(outputs: &LiveOutputs, process: u32, heard: &Mutex<Heard>) {
    let mut stream = UnixStream::connect(outputs.address()).unwrap();
    stream.write_all(&named(process, "sleep 30")).unwrap();
    stream.write_all(&frame(b"x")).unwrap();
    wait_for(heard, 1);
}

#[test]
fn ends_the_group_of_a_command_that_named_its_leader() {
    let folder = Folder::new("ends");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());
    let mut child = leader();

    serving(&outputs, &heard, || {
        named_by(&outputs, child.id(), &heard);
        outputs.end_commands(GRACE).unwrap();
    });

    assert!(child.wait().unwrap().signal().is_some());
    assert!(outputs.groups.locked().is_empty());
}

#[test]
fn never_ends_a_process_that_leads_no_group() {
    let folder = Folder::new("follower");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());
    let mut shell = command("/bin/sh");
    shell
        .args(["-c", "sleep 30 & echo $!; wait"])
        .stdout(std::process::Stdio::piped());
    let mut parent = shell.spawn().unwrap();
    let mut follower = String::new();
    let mut stdout = parent.stdout.take().unwrap();
    let mut byte = [0];
    while stdout.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
        follower.push(char::from(byte[0]));
    }
    let follower: u32 = follower.parse().unwrap();

    serving(&outputs, &heard, || {
        named_by(&outputs, follower, &heard);
        outputs.end_commands(GRACE).unwrap();
    });

    assert!(group_is_alive(parent.id()));
    stop(&mut parent).unwrap();
}

#[test]
fn forgets_the_groups_that_have_ended() {
    let folder = Folder::new("forgets");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());
    let mut child = leader();

    serving(&outputs, &heard, || {
        named_by(&outputs, child.id(), &heard);
        assert_eq!(*outputs.groups.locked(), [child.id()]);
        stop(&mut child).unwrap();
        let started = Instant::now();
        while !outputs.groups.locked().is_empty() {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "never forgotten"
            );
            thread::sleep(Duration::from_millis(20));
        }
    });

    assert!(outputs.groups.locked().is_empty());
}

#[test]
fn remembers_a_group_once_however_many_commands_name_it() {
    let folder = Folder::new("once");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let mut child = leader();

    outputs.groups.remember(child.id());
    outputs.groups.remember(child.id());
    outputs.groups.remember(NOT_A_LEADER);

    assert_eq!(*outputs.groups.locked(), [child.id()]);
    stop(&mut child).unwrap();
}

#[test]
fn stops_reading_a_command_whose_name_has_no_process() {
    let folder = Folder::new("nameless");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let heard = Mutex::new(Vec::new());

    serving(&outputs, &heard, || {
        let mut stream = UnixStream::connect(outputs.address()).unwrap();
        stream.write_all(&frame(b"ls")).unwrap();
        stream.write_all(&frame(b"never seen")).unwrap();
        thread::sleep(Duration::from_millis(200));
    });

    assert_eq!(heard.into_inner().unwrap(), Heard::new());
}

#[test]
fn stops_within_a_moment_when_no_command_ever_came() {
    let folder = Folder::new("idle");
    let outputs = LiveOutputs::open(&folder.0, "a").unwrap();
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        scope.spawn(|| {
            outputs.serve(&|_| {});
            sender.send(Instant::now()).unwrap();
        });
        thread::sleep(Duration::from_millis(150));
        let stopped = Instant::now();
        outputs.stop();
        let returned = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(returned.duration_since(stopped) < Duration::from_millis(300));
    });
}
