use std::io::Read;
use std::time::{Duration, Instant};

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);

fn output_of(mut command: Command) -> String {
    let mut child = command.stdout(Stdio::piped()).spawn().unwrap();
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    output
}

#[test]
fn removes_the_credentials_it_names() {
    let shell = command("/bin/sh");

    let removed: Vec<_> = shell
        .get_envs()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect();

    assert_eq!(removed, CREDENTIAL_VARIABLES);
}

#[test]
fn leads_a_process_group_of_its_own() {
    let mut shell = command("/bin/sh");
    shell.args(["-c", "echo $$ $(ps -o pgid= -p $$)"]);

    let output = output_of(shell);
    let ids: Vec<&str> = output.split_whitespace().collect();

    assert_eq!(ids.len(), 2, "{output}");
    assert_eq!(ids[0], ids[1]);
}

#[test]
fn stop_ends_the_program_and_what_it_started() {
    let mut shell = command("/bin/sh");
    shell
        .args(["-c", "sleep 30 & echo $!; wait"])
        .stdout(Stdio::piped());
    let mut child = shell.spawn().unwrap();
    let mut started = String::new();
    let mut stdout = child.stdout.take().unwrap();
    let mut byte = [0];
    while stdout.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
        started.push(char::from(byte[0]));
    }
    let started_at = Instant::now();

    stop(&mut child).unwrap();

    assert!(started_at.elapsed() < PATIENCE);
    assert!(child.try_wait().unwrap().is_some());
    let alive = std::process::Command::new("/bin/kill")
        .args(["-0", started.trim()])
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!alive.success(), "the background sleep still runs");
}

#[test]
fn stopping_a_program_that_already_ended_does_nothing() {
    let mut child = command("/usr/bin/true").spawn().unwrap();
    child.wait().unwrap();

    assert!(stop(&mut child).is_ok());
}

#[test]
fn killing_a_group_that_is_gone_is_no_error() {
    let mut child = command("/usr/bin/true").spawn().unwrap();
    let leader = child.id();
    child.wait().unwrap();

    assert!(kill_group(leader).is_ok());
}
