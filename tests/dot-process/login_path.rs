use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::thread;

use proptest::prelude::*;

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);
const SHORT_WAIT: Duration = Duration::from_millis(300);
const RUN_THE_SCRIPT: &str = "shift 3\nexec /bin/sh -c \"$1\"\n";
const MOST_LOOKS: usize = 100;

struct FakeShell {
    folder: PathBuf,
}

impl FakeShell {
    fn new(name: &str, body: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("dot-login-path-{}-{name}", std::process::id()));
        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        fs::create_dir_all(&folder).unwrap();
        let shell = folder.join("shell");
        let script = format!("#!/bin/sh\ncd '{}'\n{body}", folder.display());
        fs::write(&shell, script).unwrap();
        fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();
        Self { folder }
    }

    fn path(&self) -> PathBuf {
        self.folder.join("shell")
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.folder.join(name)).unwrap_or_default()
    }
}

impl Drop for FakeShell {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.folder).unwrap();
    }
}

fn printed(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| [*line, "\n"])
        .collect::<String>()
        .into_bytes()
}

fn own_path() -> OsString {
    std::env::var_os("PATH").unwrap()
}

fn is_gone(process: &str) -> bool {
    let status = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", process.trim()])
        .output()
        .unwrap();
    status.stdout.is_empty()
}

fn is_gone_soon(process: &str) -> bool {
    (0..MOST_LOOKS).any(|_| {
        thread::sleep(Duration::from_millis(10));
        is_gone(process)
    })
}

#[test]
fn reads_the_path_between_the_marks() {
    let output = printed(&[
        "Welcome back",
        MARK,
        "/opt/homebrew/bin:/usr/bin",
        MARK,
        "bye",
    ]);

    assert_eq!(
        path_between_marks(&output),
        Some(OsString::from("/opt/homebrew/bin:/usr/bin"))
    );
}

#[test]
fn takes_the_last_marks() {
    let output = printed(&[MARK, "/from/the/banner", MARK, MARK, "/usr/bin", MARK]);

    assert_eq!(
        path_between_marks(&output),
        Some(OsString::from("/usr/bin"))
    );
}

#[test]
fn refuses_marks_around_nothing() {
    assert_eq!(path_between_marks(&printed(&[MARK, "", MARK])), None);
    assert_eq!(path_between_marks(&printed(&[MARK, MARK])), None);
    assert_eq!(path_between_marks(&printed(&[MARK, MARK, MARK])), None);
}

#[test]
fn refuses_a_path_without_its_closing_mark() {
    assert_eq!(path_between_marks(&printed(&[MARK, "/usr/bin"])), None);
    assert_eq!(path_between_marks(&printed(&["/usr/bin", MARK])), None);
    assert_eq!(path_between_marks(b""), None);
}

#[test]
fn refuses_a_path_with_a_nul_byte() {
    let output = printed(&[MARK, "/usr/bin\0/bin", MARK]);

    assert_eq!(path_between_marks(&output), None);
}

proptest! {
    #[test]
    fn finds_the_path_whatever_surrounds_it(
        before in prop::collection::vec("[^\n]{0,40}", 0..8),
        path in "[^\n\0]{1,80}",
        after in prop::collection::vec("[^\n]{0,40}", 0..8),
    ) {
        prop_assume!(path != MARK && !before.iter().chain(&after).any(|line| line == MARK));
        let mut lines: Vec<&str> = before.iter().map(String::as_str).collect();
        lines.extend([MARK, path.as_str(), MARK]);
        lines.extend(after.iter().map(String::as_str));

        prop_assert_eq!(path_between_marks(&printed(&lines)), Some(OsString::from(path)));
    }
}

#[test]
fn keeps_a_short_output_whole() {
    let mut printed = Vec::new();

    keep_the_tail(&mut printed, &vec![b'x'; 2 * TAIL_KEPT]);

    assert_eq!(printed.len(), 2 * TAIL_KEPT);
}

#[test]
fn keeps_only_the_tail_of_a_long_output() {
    let mut printed = vec![b'a'; 2 * TAIL_KEPT];

    keep_the_tail(&mut printed, b"bc");

    assert_eq!(printed.len(), TAIL_KEPT);
    assert!(printed.ends_with(b"abc"));
}

#[test]
fn drains_what_is_left_without_waiting_for_the_end() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(b"left behind").unwrap();
    let mut printed = Vec::new();

    let started = Instant::now();
    drain(&mut Output::new(pipe, PIECE), &mut printed).unwrap();

    assert_eq!(printed, b"left behind");
    assert!(started.elapsed() < Duration::from_secs(1));
    drop(writer);
}

#[test]
fn drains_a_bounded_number_of_pieces() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(&[b'x'; 100]).unwrap();
    let mut printed = Vec::new();

    drain(&mut Output::new(pipe, 4), &mut printed).unwrap();

    assert_eq!(printed.len(), 4 * MOST_PIECES_AFTER_THE_END);
}

#[test]
fn asks_a_login_and_interactive_shell() {
    let shell = FakeShell::new(
        "arguments",
        &format!("printf '%s\\n' \"$@\" > arguments.txt\n{RUN_THE_SCRIPT}"),
    );

    let path = ask(&shell.path(), PATIENCE).unwrap();

    assert_eq!(path, own_path());
    assert_eq!(
        shell.read("arguments.txt"),
        format!("-l\n-i\n-c\n{}\n", script())
    );
}

#[test]
fn reads_through_what_the_shell_prints_first() {
    let shell = FakeShell::new(
        "banner",
        &format!(
            "/usr/bin/head -c 1048576 /dev/zero | /usr/bin/tr '\\0' 'x'\necho\n{RUN_THE_SCRIPT}"
        ),
    );

    assert_eq!(ask(&shell.path(), PATIENCE).unwrap(), own_path());
}

#[test]
fn reads_a_long_path_after_a_long_banner() {
    let shell = FakeShell::new(
        "long-path",
        "/usr/bin/head -c 1048576 /dev/zero | /usr/bin/tr '\\0' 'x'\necho\nlong=''\ni=0\nwhile [ $i -lt 300 ]; do long=\"$long$(printf '/opt/dot-test/folder-%03d:' $i)\"; i=$((i + 1)); done\nPATH=\"$long/usr/bin:/bin\" exec /bin/sh -c \"$4\"\n",
    );
    let long: String = (0..300)
        .map(|folder| format!("/opt/dot-test/folder-{folder:03}:"))
        .chain(["/usr/bin:/bin".to_owned()])
        .collect();

    assert_eq!(ask(&shell.path(), PATIENCE).unwrap(), OsString::from(long));
}

#[test]
fn takes_the_path_of_a_shell_that_failed_afterwards() {
    let shell = FakeShell::new("failing", "/bin/sh -c \"$4\"\nexit 1\n");

    assert_eq!(ask(&shell.path(), PATIENCE).unwrap(), own_path());
}

#[test]
fn says_when_the_shell_printed_no_path() {
    let shell = FakeShell::new("silent", "echo hello\n");

    let error = ask(&shell.path(), PATIENCE).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn says_when_the_shell_is_missing() {
    let error = ask(Path::new("/nonexistent/dot-shell-of-nobody"), PATIENCE).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn stops_a_shell_that_takes_too_long() {
    let shell = FakeShell::new("slow", "echo $$ > process.txt\nexec /bin/sleep 30\n");

    let started = Instant::now();
    let error = ask(&shell.path(), SHORT_WAIT).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(is_gone(&shell.read("process.txt")));
}

#[test]
fn stops_what_the_shell_left_running_and_does_not_wait_for_it() {
    let shell = FakeShell::new(
        "leftover",
        &format!("/bin/sleep 30 &\necho $! > leftover.txt\n{RUN_THE_SCRIPT}"),
    );

    let started = Instant::now();
    let path = ask(&shell.path(), PATIENCE).unwrap();

    assert_eq!(path, own_path());
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(is_gone_soon(&shell.read("leftover.txt")));
}

#[test]
fn reads_the_end_of_a_shell_that_ended_while_its_output_was_still_held() {
    let shell = FakeShell::new(
        "held",
        &format!(
            "/bin/sleep 30 &\n/usr/bin/head -c 40000 /dev/zero | /usr/bin/tr '\\0' 'x'\necho\n{RUN_THE_SCRIPT}"
        ),
    );

    assert_eq!(ask(&shell.path(), PATIENCE).unwrap(), own_path());
}

#[test]
fn reads_nothing_without_a_shell() {
    assert_eq!(LoginPath::new(None).read(), None);
    assert_eq!(LoginPath::default().read(), None);
}

#[test]
fn does_not_run_a_shell_named_by_a_relative_path() {
    let shell = FakeShell::new(
        "relative",
        &format!("echo run >> runs.txt\n{RUN_THE_SCRIPT}"),
    );
    let relative = shell.path().strip_prefix("/").unwrap().to_path_buf();

    assert_eq!(LoginPath::new(Some(relative)).read(), None);
    assert_eq!(shell.read("runs.txt"), "");
}

#[test]
fn runs_the_shell_once_for_every_clone_and_thread() {
    let shell = FakeShell::new(
        "once",
        &format!("echo run >> runs.txt\n/bin/sleep 0.2\n{RUN_THE_SCRIPT}"),
    );
    let login_path = LoginPath::new(Some(shell.path()));
    let clone = login_path.clone();

    let from_another_thread = thread::scope(|scope| {
        let other = scope.spawn(|| clone.read().map(OsStr::to_owned));
        let here = login_path.read().map(OsStr::to_owned);
        assert_eq!(here.as_deref(), Some(own_path().as_os_str()));
        other.join().unwrap()
    });

    assert_eq!(from_another_thread, Some(own_path()));
    assert_eq!(
        login_path.read().map(OsStr::as_bytes),
        Some(own_path().as_bytes())
    );
    assert_eq!(shell.read("runs.txt"), "run\n");
}

#[test]
#[ignore = "runs the shell configuration of whoever runs the tests"]
fn reads_the_path_of_the_real_zsh() {
    let path = ask(Path::new("/bin/zsh"), Duration::from_secs(10)).unwrap();

    assert!(std::env::split_paths(&path).any(|folder| folder == Path::new("/usr/bin")));
}
