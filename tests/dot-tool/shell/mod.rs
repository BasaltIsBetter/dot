use std::fs;
use std::time::Instant;

use dot_api::{ToolContent, ToolEnd};

use super::*;
use crate::toolbox::tests::TempFolder;

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn run(shell: &Shell, command: &str) -> ToolOutput {
    shell.run(command, &Cancellation::new())
}

#[test]
fn runs_in_the_folder_and_merges_errors_into_the_output() {
    let folder = TempFolder::new("shell-run");
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    let output = run(&shell, "pwd; echo oops >&2");

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
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&shell, "cd sub");
    let moved = run(&shell, "pwd");
    let restarted = shell.restart();
    let back = run(&shell, "pwd");

    assert_eq!(text(&moved), format!("{}\n", folder.join("sub").display()));
    assert_eq!(restarted.end, ToolEnd::Done(Detail::Nothing));
    assert!(text(&restarted).contains(&folder.path().display().to_string()));
    assert_eq!(text(&back), format!("{}\n", folder.path().display()));
}

#[test]
fn a_working_folder_removed_since_falls_back_to_the_session_s() {
    let folder = TempFolder::new("shell-gone");
    fs::create_dir(folder.join("gone")).unwrap();
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&shell, "cd gone");
    fs::remove_dir(folder.join("gone")).unwrap();
    let output = run(&shell, "pwd");

    assert_eq!(text(&output), format!("{}\n", folder.path().display()));
}

#[test]
fn shell_variables_do_not_carry_over() {
    let shell = Shell::new(None);

    run(&shell, "export DOT_TEST_VALUE=kept");
    let output = run(&shell, "echo \"[${DOT_TEST_VALUE-}]\"");

    assert_eq!(text(&output), "[]\n");
}

#[test]
fn without_a_folder_commands_start_at_home() {
    let shell = Shell::new(None);

    let output = run(&shell, "test \"$PWD\" = \"$HOME\" && echo home");

    assert_eq!(text(&output), "home\n");
    assert!(text(&shell.restart()).contains("home folder"));
}

#[test]
fn a_failing_command_still_ran_and_tells_its_status() {
    let shell = Shell::new(None);

    let output = run(&shell, "echo partial; exit 2");

    assert_eq!(text(&output), "partial\n\n[exit status 2]");
    assert_eq!(output.end, ToolEnd::Done(Detail::Exit(2)));
    assert!(!output.is_error);
}

#[test]
fn says_when_a_command_printed_nothing() {
    let shell = Shell::new(None);

    assert_eq!(text(&run(&shell, "true")), NO_OUTPUT);
    assert_eq!(text(&run(&shell, "false")), "(no output)\n[exit status 1]");
}

#[test]
fn a_command_past_its_time_is_an_error_with_its_output() {
    let shell = Shell::with_timeout(None, Duration::from_millis(200));

    let output = run(&shell, "echo begun; sleep 30");

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(output.is_error);
    assert!(text(&output).contains("longer than 0 seconds"));
    assert!(text(&output).ends_with("begun\n"));
}

#[test]
fn a_stopped_command_reports_the_stop() {
    let shell = Shell::new(None);
    let cancellation = Cancellation::new();
    let cancelling = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        cancelling.cancel();
    });
    let started = Instant::now();

    let output = shell.run("sleep 30", &cancellation);

    canceller.join().unwrap();
    assert_eq!(output, ToolOutput::stopped());
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn an_exit_inside_the_command_still_reports_the_folder() {
    let folder = TempFolder::new("shell-exit");
    fs::create_dir(folder.join("sub")).unwrap();
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&shell, "cd sub; exit 0");
    let output = run(&shell, "pwd");

    assert_eq!(text(&output), format!("{}\n", folder.join("sub").display()));
}

#[test]
fn caps_a_flood_of_output() {
    let shell = Shell::new(None);

    let output = run(&shell, "yes | head -c 200000");

    assert!(text(&output).len() < 31_000);
    assert!(text(&output).contains("bytes of output cut here"));
}

#[test]
fn closing_the_shell_stops_what_it_left_running() {
    let folder = TempFolder::new("shell-drop");
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&shell, "(sleep 1; touch survived) &");
    drop(shell);
    std::thread::sleep(Duration::from_millis(1_500));

    assert!(!folder.join("survived").exists());
}

#[test]
fn a_restart_stops_what_earlier_commands_left_running() {
    let folder = TempFolder::new("shell-restart-stop");
    let shell = Shell::new(Some(folder.path().to_path_buf()));

    run(&shell, "(sleep 1; touch survived) &");
    shell.restart();
    std::thread::sleep(Duration::from_millis(1_500));

    assert!(!folder.join("survived").exists());
}

#[test]
fn a_finished_background_child_is_forgotten() {
    let shell = Shell::new(None);

    run(&shell, "(sleep 0.2; echo late) &");
    std::thread::sleep(Duration::from_millis(400));
    run(&shell, "true");

    assert_eq!(shell.state().lingering.len(), 0);
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
