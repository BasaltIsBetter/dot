use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);
const QUICK: Duration = Duration::from_millis(400);
const IGNORES_THE_REQUEST: &str =
    "trap '' TERM; echo ready; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done";

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

fn started(script: &str) -> (Child, String) {
    let mut shell = command("/bin/sh");
    shell.args(["-c", script]).stdout(Stdio::piped());
    let mut child = shell.spawn().unwrap();
    let mut first_line = String::new();
    let mut stdout = child.stdout.take().unwrap();
    let mut byte = [0];
    while stdout.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
        first_line.push(char::from(byte[0]));
    }
    (child, first_line)
}

fn scratch(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("dot-launch-{name}-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    fs::create_dir_all(&folder).unwrap();
    folder
}

fn is_running(process: &str) -> bool {
    let alive = || {
        std::process::Command::new("/bin/kill")
            .args(["-0", process.trim()])
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    let started_at = Instant::now();
    while alive() {
        if started_at.elapsed() > PATIENCE {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
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
    let (mut child, background) = started("sleep 30 & echo $!; wait");
    let started_at = Instant::now();

    stop(&mut child).unwrap();

    assert!(started_at.elapsed() < PATIENCE);
    assert!(child.try_wait().unwrap().is_some());
    assert!(!is_running(&background), "the background sleep still runs");
}

#[test]
fn stop_returns_as_soon_as_the_group_has_ended() {
    let (mut child, _) = started("echo ready; exec sleep 30");
    let started_at = Instant::now();

    stop(&mut child).unwrap();

    assert!(started_at.elapsed() < QUICK, "{:?}", started_at.elapsed());
    assert!(!group_is_alive(child.id()));
}

#[test]
fn stop_lets_the_program_clean_up_before_it_ends() {
    let folder = scratch("clean-up");
    let cleaned = folder.join("cleaned");
    let script = format!(
        "trap 'echo done > {}; exit 0' TERM; echo ready; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done",
        cleaned.display()
    );
    let (mut child, _) = started(&script);

    stop(&mut child).unwrap();

    assert_eq!(fs::read_to_string(&cleaned).unwrap(), "done\n");
    drop(fs::remove_dir_all(folder));
}

#[test]
fn stop_kills_a_program_that_ignores_the_request_after_the_grace() {
    let (mut child, _) = started(IGNORES_THE_REQUEST);
    let started_at = Instant::now();

    stop(&mut child).unwrap();

    let took = started_at.elapsed();
    assert!(took >= GRACE, "{took:?}");
    assert!(took < GRACE + PATIENCE, "{took:?}");
    assert!(child.try_wait().unwrap().is_some());
    assert!(!group_is_alive(child.id()));
}

#[test]
fn stop_ends_what_the_program_left_after_it_ended() {
    let (mut child, background) = started("trap '' TERM; sleep 30 & echo $!; exit 0");
    let started_at = Instant::now();
    while child.try_wait().unwrap().is_none() && started_at.elapsed() < PATIENCE {
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(group_is_alive(child.id()));
    assert!(end_groups(&[child.id()], GRACE).is_ok());
    assert!(!is_running(&background), "the background sleep still runs");
}

#[test]
fn stop_waits_for_the_whole_group_and_not_only_the_program() {
    let (mut child, stubborn) = started(
        "(trap '' TERM; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done) & echo $!; trap 'exit 0' TERM; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done",
    );

    stop(&mut child).unwrap();

    assert!(child.try_wait().unwrap().is_some());
    assert!(
        !is_running(&stubborn),
        "what ignores the request still runs"
    );
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
    assert!(terminate_group(leader).is_ok());
    assert!(!group_is_alive(leader));
}

#[test]
fn never_takes_group_zero_one_or_its_own_for_a_group_of_dot() {
    for leader in [0, 1, own_group(), u32::MAX] {
        assert_eq!(
            group(leader).unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "{leader}"
        );
        assert!(!group_is_alive(leader), "{leader}");
        assert_eq!(group_led_by(leader), None, "{leader}");
    }
}

#[test]
fn signals_no_group_it_refuses() {
    assert_eq!(
        kill_group(u32::MAX).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        terminate_group(u32::MAX).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert!(end_groups(&[u32::MAX], Duration::ZERO).is_err());
}

#[test]
fn sees_a_group_alive_while_one_of_its_processes_runs() {
    let mut child = command("/bin/sleep").arg("30").spawn().unwrap();

    assert!(group_is_alive(child.id()));

    kill_group(child.id()).unwrap();
    child.wait().unwrap();
    assert!(!group_is_alive(child.id()));
}

#[test]
fn tells_a_group_leader_from_a_follower() {
    let (mut child, follower) = started("sleep 30 & echo $!; wait");
    let follower: u32 = follower.trim().parse().unwrap();

    assert_eq!(group_led_by(child.id()), Some(child.id()));
    assert_eq!(group_led_by(follower), None);

    stop(&mut child).unwrap();
    assert_eq!(group_led_by(child.id()), None);
}

#[test]
fn knows_its_own_group() {
    let mut shell = Command::new("/bin/ps");
    shell.args(["-o", "pgid=", "-p", &std::process::id().to_string()]);

    assert_eq!(output_of(shell).trim(), own_group().to_string());
}

#[test]
fn ends_several_groups_within_one_grace() {
    let (stubborn, _) = started(IGNORES_THE_REQUEST);
    let (willing, _) = started("echo ready; exec sleep 30");
    let mut children = [stubborn, willing];
    let leaders: Vec<u32> = children.iter().map(Child::id).collect();
    let started_at = Instant::now();

    end_groups(&leaders, GRACE).unwrap();

    let took = started_at.elapsed();
    assert!(took >= GRACE && took < GRACE + PATIENCE, "{took:?}");
    for child in &mut children {
        child.wait().unwrap();
    }
    assert!(leaders.iter().all(|&leader| !group_is_alive(leader)));
}

#[test]
fn asks_a_group_again_every_tenth_of_a_second_and_no_more_often() {
    let folder = scratch("asked");
    let asked = folder.join("asked");
    let script = format!(
        "trap 'echo asked >> {}' TERM; echo ready; i=0; while [ $i -lt 1500 ]; do i=$((i+1)); sleep 0.02; done",
        asked.display()
    );
    let (mut child, _) = started(&script);

    end_groups(&[child.id()], GRACE).unwrap();
    child.wait().unwrap();

    let times_asked = fs::read_to_string(&asked).unwrap().lines().count();
    let most_asks_in_the_grace = 6;
    assert!(
        (2..=most_asks_in_the_grace).contains(&times_asked),
        "{times_asked}"
    );
    drop(fs::remove_dir_all(folder));
}

#[test]
fn ending_groups_that_end_willingly_does_not_wait_for_the_grace() {
    let (mut willing, _) = started("echo ready; exec sleep 30");
    let leader = willing.id();
    let reaper = std::thread::spawn(move || willing.wait());
    let started_at = Instant::now();

    end_groups(&[leader], PATIENCE).unwrap();

    assert!(
        started_at.elapsed() < QUICK + QUICK,
        "{:?}",
        started_at.elapsed()
    );
    reaper.join().unwrap().unwrap();
}

#[test]
fn ending_no_group_returns_at_once() {
    let started_at = Instant::now();

    assert!(end_groups(&[], PATIENCE).is_ok());
    assert!(started_at.elapsed() < QUICK);
}

#[test]
fn ends_a_group_whose_process_was_being_started_when_it_was_asked() {
    for _ in 0..20 {
        let (mut child, _) = started("echo ready; sleep 30");
        let leader = child.id();
        let started_at = Instant::now();

        end_groups(&[leader], GRACE).unwrap();

        assert!(started_at.elapsed() < GRACE, "{:?}", started_at.elapsed());
        child.wait().unwrap();
    }
}

#[test]
fn stop_ends_a_program_being_started_when_it_was_asked() {
    for _ in 0..20 {
        let (mut child, _) = started("echo ready; sleep 30");
        let started_at = Instant::now();

        stop(&mut child).unwrap();

        assert!(started_at.elapsed() < GRACE, "{:?}", started_at.elapsed());
        assert!(!group_is_alive(child.id()));
    }
}
