use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Stdio;

use super::*;

const QUICK_GRACE: Duration = Duration::from_millis(300);

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", script])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

#[test]
fn hands_over_the_pipes_of_the_program() {
    let (running, pipes) = Running::start(shell("echo hello"), &Cancellation::new()).unwrap();
    let mut printed = String::new();

    pipes.stdout.unwrap().read_to_string(&mut printed).unwrap();

    assert_eq!(printed, "hello\n");
    assert!(pipes.stdin.is_some());
    assert!(running.finish(Duration::from_secs(5)).unwrap().success());
}

#[test]
fn stops_the_whole_group_once_cancelled() {
    let cancellation = Cancellation::new();
    let (running, pipes) = Running::start(shell("sleep 30 & sleep 30"), &cancellation).unwrap();
    let started = Instant::now();

    cancellation.cancel();
    let mut printed = String::new();
    pipes.stdout.unwrap().read_to_string(&mut printed).unwrap();
    let status = running.finish(Duration::from_secs(5)).unwrap();

    assert!(!status.success());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn stops_a_program_that_outstays_its_grace() {
    let (running, _pipes) = Running::start(shell("sleep 30"), &Cancellation::new()).unwrap();
    let started = Instant::now();

    let status = running.finish(Duration::from_millis(100)).unwrap();

    assert!(!status.success());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn leaves_a_finished_program_alone_when_cancelled_late() {
    let cancellation = Cancellation::new();
    let (running, _pipes) = Running::start(shell("exit 0"), &cancellation).unwrap();
    let status = running.finish(Duration::from_secs(5)).unwrap();

    cancellation.cancel();

    assert!(status.success());
}

#[test]
fn reports_a_program_that_cannot_start() {
    let missing = Command::new("/nonexistent/claude");

    assert!(Running::start(missing, &Cancellation::new()).is_err());
}

#[test]
fn polls_twice_as_slowly_each_time_up_to_its_longest_pause() {
    assert_eq!(longer(FIRST_POLL), Duration::from_millis(2));
    assert_eq!(longer(Duration::from_millis(32)), LONGEST_POLL);
    assert_eq!(longer(LONGEST_POLL), LONGEST_POLL);
}

fn scratch(name: &str) -> std::path::PathBuf {
    let folder =
        std::env::temp_dir().join(format!("dot-model-process-{name}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&folder));
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn first_line(pipes: &mut Pipes) -> String {
    let mut line = String::new();
    let mut byte = [0];
    let stdout = pipes.stdout.as_mut().unwrap();
    while stdout.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
        line.push(char::from(byte[0]));
    }
    line
}

#[test]
fn asks_the_program_to_end_when_cancelled_and_lets_it_clean_up() {
    let folder = scratch("clean-up");
    let cleaned = folder.join("cleaned");
    let script = format!(
        "trap 'echo done > {}; exit 0' TERM; echo ready; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done",
        cleaned.display()
    );
    let cancellation = Cancellation::new();
    let (running, mut pipes) = Running::start(shell(&script), &cancellation).unwrap();
    first_line(&mut pipes);

    cancellation.cancel();
    running.finish(CANCEL_GRACE).unwrap();

    assert_eq!(std::fs::read_to_string(&cleaned).unwrap(), "done\n");
    drop(std::fs::remove_dir_all(folder));
}

#[test]
fn kills_a_program_that_outstays_the_grace_after_a_cancel() {
    let cancellation = Cancellation::new();
    let script =
        "trap '' TERM; echo ready; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done";
    let (running, mut pipes) = Running::start(shell(script), &cancellation).unwrap();
    first_line(&mut pipes);
    let finished = AtomicBool::new(false);
    let started = Instant::now();

    cancellation.cancel();
    running.end_when_cancelled(&cancellation, &finished, QUICK_GRACE);

    let took = started.elapsed();
    assert!(
        took >= QUICK_GRACE && took < QUICK_GRACE + Duration::from_secs(2),
        "{took:?}"
    );
    assert!(!running.finish(Duration::from_secs(5)).unwrap().success());
}

#[test]
fn gives_claude_code_two_seconds_to_end_after_a_cancel() {
    assert_eq!(CANCEL_GRACE, Duration::from_secs(2));
}

#[test]
fn stops_watching_once_the_reply_has_finished() {
    let cancellation = Cancellation::new();
    let (running, _pipes) = Running::start(shell("sleep 30"), &cancellation).unwrap();
    let finished = AtomicBool::new(true);
    let started = Instant::now();

    running.end_when_cancelled(&cancellation, &finished, CANCEL_GRACE);

    assert!(started.elapsed() < Duration::from_millis(100));
    assert!(dot_process::group_is_alive(running.leader));
}

#[test]
fn never_kills_a_program_while_nothing_is_cancelled() {
    let cancellation = Cancellation::new();
    let (running, _pipes) = Running::start(shell("sleep 30"), &cancellation).unwrap();
    let finished = AtomicBool::new(false);

    thread::scope(|scope| {
        scope.spawn(|| running.end_when_cancelled(&cancellation, &finished, QUICK_GRACE));
        thread::sleep(QUICK_GRACE + Duration::from_millis(200));
        assert!(dot_process::group_is_alive(running.leader));
        finished.store(true, Ordering::Release);
    });
}

#[test]
fn ends_what_the_program_left_running_in_its_group() {
    let (running, mut pipes) =
        Running::start(shell("sleep 30 & echo $!; exit 0"), &Cancellation::new()).unwrap();
    let left = first_line(&mut pipes);

    assert!(running.finish(Duration::from_secs(5)).unwrap().success());

    assert!(!dot_process::group_is_alive(running.leader));
    let alive = Command::new("/bin/kill")
        .args(["-0", left.trim()])
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!alive.success(), "what the program left still runs");
}

#[test]
fn stops_the_program_when_dropped_before_it_ended() {
    let (running, _pipes) = Running::start(shell("sleep 30"), &Cancellation::new()).unwrap();
    let leader = running.leader;

    drop(running);

    assert!(!dot_process::group_is_alive(leader));
}

#[test]
fn hands_the_prompt_over_as_one_line_of_json() {
    let (running, mut pipes) = Running::start(shell("head -n 1"), &Cancellation::new()).unwrap();

    let prompt = Prompt {
        text: "Salut".to_owned(),
        images: Vec::new(),
    };
    hand_over(pipes.stdin.as_mut(), &prompt).unwrap();
    drop(pipes.stdin.take());
    let line = first_line(&mut pipes);

    assert_eq!(
        line,
        r#"{"type":"user","message":{"role":"user","content":"Salut"}}"#
    );
    running.finish(Duration::from_secs(5)).unwrap();
}
