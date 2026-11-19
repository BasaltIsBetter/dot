use std::ffi::OsString;

use super::*;

const HOME: &str = "/Users/someone";
const PROGRAM: &str = "tool";
const HOME_DIRECTORIES: [&str; 1] = [".local/bin"];
const SHORT_WAIT: Duration = Duration::from_millis(200);
const LONG_WAIT: Duration = Duration::from_secs(5);

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

fn shell_output(script: &str, longest: u64) -> Result<String, RunError> {
    Program::new(PathBuf::from("/bin/sh")).output(&["-c", script], LONG_WAIT, longest)
}

#[test]
fn finds_a_program_in_the_path_first() {
    let path = OsString::from("/usr/bin:/opt/tools/bin");

    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        Some(Path::new(HOME)),
        Some(&path),
        |candidate| candidate.starts_with("/opt/tools") || candidate.starts_with(HOME),
    );

    assert_eq!(found, Some(PathBuf::from("/opt/tools/bin/tool")));
}

#[test]
fn finds_a_program_where_its_installer_puts_it() {
    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        Some(Path::new(HOME)),
        Some(OsStr::new("/usr/bin:/bin")),
        found_in("/Users/someone/.local/bin/tool"),
    );

    assert_eq!(found, Some(PathBuf::from("/Users/someone/.local/bin/tool")));
}

#[test]
fn finds_a_program_installed_by_homebrew_without_home() {
    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        None,
        None,
        found_in("/opt/homebrew/bin/tool"),
    );

    assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/tool")));
}

#[test]
fn ignores_relative_directories_of_the_path() {
    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        None,
        Some(OsStr::new("bin:.")),
        |_| true,
    );

    assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/tool")));
}

#[test]
fn finds_nothing_when_the_program_is_not_installed() {
    let found = locate(
        PROGRAM,
        &HOME_DIRECTORIES,
        Some(Path::new(HOME)),
        None,
        |_| false,
    );

    assert_eq!(found, None);
}

#[test]
fn returns_the_exit_status_of_a_finished_program() {
    let child = Command::new("/usr/bin/false").spawn().unwrap();

    let status = wait(child, LONG_WAIT, &Cancellation::new()).unwrap();

    assert!(!status.success());
}

#[test]
fn stops_the_program_once_cancelled() {
    let cancellation = Cancellation::new();
    cancellation.cancel();
    let child = sleeper();
    let process = child.id();

    let waited = wait(child, LONG_WAIT, &cancellation);

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
fn notices_a_quick_program_without_waiting_long() {
    let started = Instant::now();
    let child = Command::new("/usr/bin/true").spawn().unwrap();

    wait(child, LONG_WAIT, &Cancellation::new()).unwrap();

    assert!(started.elapsed() < LONGEST_POLL);
}

#[test]
fn runs_a_program_until_it_succeeds_or_fails() {
    let succeeding = Program::new(PathBuf::from("/usr/bin/true"));
    let failing = Program::new(PathBuf::from("/usr/bin/false"));

    assert!(succeeding.run(&[], LONG_WAIT, &Cancellation::new()).is_ok());
    assert!(matches!(
        failing.run(&[], LONG_WAIT, &Cancellation::new()),
        Err(RunError::Failed(_))
    ));
}

#[test]
fn reports_a_program_that_cannot_start() {
    let missing = Program::new(PathBuf::from("/nonexistent/tool"));

    let error = missing
        .run(&[], LONG_WAIT, &Cancellation::new())
        .unwrap_err();

    assert!(matches!(error, RunError::Spawn(..)));
    assert!(error.to_string().contains("/nonexistent/tool"));
}

#[test]
fn reads_what_the_program_prints() {
    let printed = shell_output("printf 'hello'", 1_024).unwrap();

    assert_eq!(printed, "hello");
}

#[test]
fn reads_no_more_than_allowed() {
    let printed = shell_output("printf 'hello'", 3).unwrap();

    assert_eq!(printed, "hel");
}

#[test]
fn reads_nothing_from_a_program_that_fails() {
    let failed = shell_output("printf 'hello'; exit 3", 1_024);

    assert!(matches!(failed, Err(RunError::Failed(_))));
}

#[test]
fn hides_anthropic_credentials_of_the_environment_from_the_program() {
    let command = Program::new(PathBuf::from("/bin/sh")).command(&[], Stdio::null());

    let removed: Vec<_> = command
        .get_envs()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| name.to_owned())
        .collect();

    assert_eq!(removed, CREDENTIAL_VARIABLES.map(OsString::from));
}

#[test]
fn waits_twice_as_long_each_time_up_to_its_longest_pause() {
    assert_eq!(longer(FIRST_POLL), Duration::from_millis(2));
    assert_eq!(longer(Duration::from_millis(16)), Duration::from_millis(32));
    assert_eq!(longer(Duration::from_millis(32)), LONGEST_POLL);
    assert_eq!(longer(LONGEST_POLL), LONGEST_POLL);
}
