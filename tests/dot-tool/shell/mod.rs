use std::fs;
use std::time::Instant;

use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::toolbox::tests::{FAKE_LOGIN_PATH, TempFolder, fake_login_shell};

const STOPPED_WITHIN: Duration = Duration::from_secs(3);

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn run(shell: &mut Shell, command: &str) -> ToolOutput {
    shell.run(command, &Cancellation::new(), &mut |_| {})
}

#[test]
fn finds_programs_where_the_login_shell_finds_them() {
    let folder = TempFolder::new("shell-login-path");
    let mut shell = Shell::new(None).login_path(fake_login_shell(&folder));

    let output = run(&mut shell, "printenv PATH");

    assert_eq!(text(&output), format!("{FAKE_LOGIN_PATH}\n"));
}

#[test]
fn keeps_dot_s_own_path_without_a_login_shell() {
    let mut shell = Shell::new(None);

    let output = run(&mut shell, "printenv PATH");

    let own = std::env::var("PATH").unwrap();
    assert_eq!(text(&output), format!("{own}\n"));
}

#[test]
fn runs_in_the_folder_and_merges_errors_into_the_output() {
    let folder = TempFolder::new("shell-run");
    let mut shell = Shell::new(Some(folder.path().to_path_buf()));

    let output = run(&mut shell, "pwd; echo oops >&2");

    assert_eq!(
        text(&output),
        format!("{}\noops\n", folder.path().display())
    );
    assert_eq!(output.end, ToolEnd::Done(Detail::Exit(0)));
    assert!(!output.is_error);
}

#[test]
fn the_working_folder_carries_over_to_the_next_command_until_a_restart() {
    let folder = TempFolder::new("shell-cd");
    fs::create_dir(folder.join("sub")).unwrap();
    let mut shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&mut shell, "cd sub");
    let moved = run(&mut shell, "pwd");
    let restarted = shell.restart();
    let back = run(&mut shell, "pwd");

    assert_eq!(text(&moved), format!("{}\n", folder.join("sub").display()));
    assert_eq!(restarted.end, ToolEnd::Done(Detail::Nothing));
    assert!(text(&restarted).contains(&folder.path().display().to_string()));
    assert_eq!(text(&back), format!("{}\n", folder.path().display()));
}

#[test]
fn a_working_folder_removed_since_falls_back_to_the_session_s() {
    let folder = TempFolder::new("shell-gone");
    fs::create_dir(folder.join("gone")).unwrap();
    let mut shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&mut shell, "cd gone");
    fs::remove_dir(folder.join("gone")).unwrap();
    let output = run(&mut shell, "pwd");

    assert_eq!(text(&output), format!("{}\n", folder.path().display()));
}

#[test]
fn shell_variables_do_not_carry_over() {
    let mut shell = Shell::new(None);

    run(&mut shell, "export DOT_TEST_VALUE=kept");
    let output = run(&mut shell, "echo \"[${DOT_TEST_VALUE-}]\"");

    assert_eq!(text(&output), "[]\n");
}

#[test]
fn without_a_folder_commands_start_at_home() {
    let mut shell = Shell::new(None);

    let output = run(&mut shell, "test \"$PWD\" = \"$HOME\" && echo home");

    assert_eq!(text(&output), "home\n");
    assert!(text(&shell.restart()).contains("home folder"));
}

#[test]
fn a_failing_command_still_ran_and_tells_its_status() {
    let mut shell = Shell::new(None);

    let output = run(&mut shell, "echo partial; exit 2");

    assert_eq!(text(&output), "partial\n\n[exit status 2]");
    assert_eq!(output.end, ToolEnd::Done(Detail::Exit(2)));
    assert!(!output.is_error);
}

#[test]
fn says_when_a_command_printed_nothing() {
    let mut shell = Shell::new(None);

    assert_eq!(text(&run(&mut shell, "true")), NO_OUTPUT);
    assert_eq!(
        text(&run(&mut shell, "false")),
        "(no output)\n[exit status 1]"
    );
}

#[test]
fn a_command_past_its_time_is_an_error_with_its_output() {
    let mut shell = Shell::with_timeout(None, Duration::from_millis(200));

    let output = run(&mut shell, "echo begun; sleep 30");

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(output.is_error);
    assert!(text(&output).contains("longer than 0 seconds"));
    assert!(text(&output).ends_with("begun\n"));
}

#[test]
fn a_stopped_command_reports_the_stop() {
    let mut shell = Shell::new(None);
    let cancellation = Cancellation::new();
    let cancelling = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        cancelling.cancel();
    });
    let started = Instant::now();

    let output = shell.run("sleep 30", &cancellation, &mut |_| {});

    canceller.join().unwrap();
    assert_eq!(output, ToolOutput::stopped());
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn an_exit_inside_the_command_still_reports_the_folder() {
    let folder = TempFolder::new("shell-exit");
    fs::create_dir(folder.join("sub")).unwrap();
    let mut shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&mut shell, "cd sub; exit 0");
    let output = run(&mut shell, "pwd");

    assert_eq!(text(&output), format!("{}\n", folder.join("sub").display()));
}

#[test]
fn caps_a_flood_of_output() {
    let mut shell = Shell::new(None);

    let output = run(&mut shell, "yes | head -c 200000");

    assert!(text(&output).len() < 31_000);
    assert!(text(&output).contains("bytes of output cut here"));
}

fn left_running(shell: &mut Shell) -> String {
    text(&run(shell, "sleep 30 & echo $!")).trim().to_owned()
}

fn ends_soon(process: &str) -> bool {
    let alive = || {
        std::process::Command::new("/bin/kill")
            .args(["-0", process])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    let started = Instant::now();
    while alive() {
        if started.elapsed() > STOPPED_WITHIN {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
fn closing_the_shell_stops_what_it_left_running() {
    let mut shell = Shell::new(None);
    let process = left_running(&mut shell);

    drop(shell);

    assert!(ends_soon(&process), "{process} still runs");
}

#[test]
fn a_restart_stops_what_earlier_commands_left_running() {
    let mut shell = Shell::new(None);
    let process = left_running(&mut shell);

    shell.restart();

    assert!(ends_soon(&process), "{process} still runs");
}

#[test]
fn a_group_ended_by_someone_else_is_forgotten() {
    let mut shell = Shell::new(None);

    run(&mut shell, "(sleep 30; echo late) &");
    let group = shell.lingering[0].group;
    dot_process::kill_group(group).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    run(&mut shell, "true");

    assert_eq!(shell.lingering.len(), 0);
}

#[test]
fn a_background_child_keeps_its_group_until_the_shell_restarts() {
    let mut shell = Shell::new(None);

    run(&mut shell, "(sleep 0.2; echo late) &");
    std::thread::sleep(Duration::from_millis(400));
    run(&mut shell, "true");

    assert_eq!(shell.lingering.len(), 1);
    shell.restart();
    assert_eq!(shell.lingering.len(), 0);
}

#[test]
fn writes_a_script_that_reports_the_folder_on_exit() {
    let script = script("ls", "__S__", false);
    let homeward = script_with_home();

    assert_eq!(
        script,
        "exec 2>&1\ntrap 'printf \"\\n%s%s\\n\" \"__S__\" \"$PWD\"' EXIT\n\nls\n"
    );
    assert!(homeward.contains(GO_HOME));
}

fn script_with_home() -> String {
    script("ls", "__S__", true)
}

#[test]
fn draws_a_sentinel_unlikely_to_appear_in_an_output() {
    let first = sentinel();
    let second = sentinel();

    assert!(first.starts_with(SENTINEL_PREFIX));
    assert!(first.ends_with(SENTINEL_SUFFIX));
    assert_eq!(
        first.len(),
        SENTINEL_PREFIX.len() + 32 + SENTINEL_SUFFIX.len()
    );
    assert_ne!(first, second);
}

#[test]
fn shows_what_comes_before_the_folder_sentinel_and_nothing_of_it() {
    let piece = b"done\n\n__DOT_FOLDER_0123456789abcdef__/Users/me\n";

    assert_eq!(before_the_sentinel(piece), b"done\n\n");
    assert_eq!(before_the_sentinel(b"plain\n"), b"plain\n");
    assert_eq!(before_the_sentinel(b"__DOT_FOLDER_x"), b"");
}

#[test]
fn hands_the_output_of_a_running_command_out() {
    let mut shell = Shell::new(None);
    let mut shown = Vec::new();

    let output = shell.run(
        "echo early; sleep 0.6; echo late",
        &Cancellation::new(),
        &mut |piece| shown.extend_from_slice(piece),
    );

    assert_eq!(text(&output), "early\nlate\n");
    assert!(shown.starts_with(b"early\n"), "{shown:?}");
    assert!(!String::from_utf8_lossy(&shown).contains("__DOT_FOLDER_"));
}
