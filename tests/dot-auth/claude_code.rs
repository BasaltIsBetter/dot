use std::ffi::OsString;

use super::*;

const HOME: &str = "/Users/someone";
const SHORT_WAIT: Duration = Duration::from_millis(200);

fn found_in(installed: &'static str) -> impl Fn(&Path) -> bool {
    move |candidate| candidate == Path::new(installed)
}

fn sleeper() -> Child {
    Command::new("/bin/sleep").arg("30").spawn().unwrap()
}

fn is_running(process: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &process.to_string()])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[test]
fn finds_an_installed_claude_code_on_disk() {
    let directory = std::env::temp_dir().join(format!("dot-locate-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join(PROGRAM), b"").unwrap();

    let found = locate_claude_code(None, Some(directory.as_os_str()));

    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(found, Some(directory.join(PROGRAM)));
}

#[test]
fn finds_claude_code_in_the_path_first() {
    let path = OsString::from("/usr/bin:/opt/tools/bin");

    let found = locate(Some(Path::new(HOME)), Some(&path), |candidate| {
        candidate.starts_with("/opt/tools") || candidate.starts_with(HOME)
    });

    assert_eq!(found, Some(PathBuf::from("/opt/tools/bin/claude")));
}

#[test]
fn finds_claude_code_where_its_installer_puts_it() {
    let found = locate(
        Some(Path::new(HOME)),
        Some(OsStr::new("/usr/bin:/bin")),
        found_in("/Users/someone/.local/bin/claude"),
    );

    assert_eq!(
        found,
        Some(PathBuf::from("/Users/someone/.local/bin/claude"))
    );
}

#[test]
fn finds_claude_code_installed_by_homebrew_without_home() {
    let found = locate(None, None, found_in("/opt/homebrew/bin/claude"));

    assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/claude")));
}

#[test]
fn ignores_relative_directories_of_the_path() {
    let found = locate(None, Some(OsStr::new("bin:.")), |_| true);

    assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/claude")));
}

#[test]
fn finds_nothing_when_claude_code_is_not_installed() {
    assert_eq!(locate(Some(Path::new(HOME)), None, |_| false), None);
}

#[test]
fn returns_the_exit_status_of_a_finished_program() {
    let child = Command::new("/usr/bin/false").spawn().unwrap();

    let status = wait(child, SHORT_WAIT * 10, &Cancellation::new()).unwrap();

    assert!(!status.success());
}

#[test]
fn stops_the_program_once_cancelled() {
    let cancellation = Cancellation::new();
    cancellation.cancel();
    let child = sleeper();
    let process = child.id();

    let waited = wait(child, SHORT_WAIT * 10, &cancellation);

    assert!(matches!(waited, Err(RunError::Cancelled)));
    assert!(!is_running(process));
}

#[test]
fn stops_the_program_after_its_timeout() {
    let started = Instant::now();
    let child = sleeper();
    let process = child.id();

    let waited = wait(child, SHORT_WAIT, &Cancellation::new());

    assert!(matches!(waited, Err(RunError::TimedOut(SHORT_WAIT))));
    assert!(!is_running(process));
    assert!(started.elapsed() < SHORT_WAIT * 5);
}

#[test]
fn a_program_that_succeeds_signs_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/usr/bin/true"));

    assert!(cli.is_signed_in().unwrap());
    assert!(cli.sign_in(&Cancellation::new()).is_ok());
}

#[test]
fn a_program_that_fails_is_signed_out_and_fails_to_sign_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/usr/bin/false"));

    assert!(!cli.is_signed_in().unwrap());
    assert_eq!(
        cli.sign_in(&Cancellation::new()).unwrap_err().failure(),
        ConnectFailure::ClaudeCodeFailed
    );
}

#[test]
fn a_missing_program_fails_to_sign_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/nonexistent/claude"));

    assert_eq!(
        cli.is_signed_in().unwrap_err().failure(),
        ConnectFailure::ClaudeCodeFailed
    );
}
