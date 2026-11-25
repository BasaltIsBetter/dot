use super::*;

fn group() -> Pid {
    Pid::from_raw(42).unwrap()
}

#[test]
fn a_running_process_keeps_its_group_alive() {
    assert!(runs_in("43 (sleep) S 1 42 42 0 -1", group()));
    assert!(!runs_in("43 (sleep) S 1 41 42 0 -1", group()));
}

#[test]
fn an_ended_process_does_not() {
    assert!(!runs_in("42 (sh) Z 1 42 42 0 -1", group()));
    assert!(!runs_in("42 (sh) X 1 42 42 0 -1", group()));
}

#[test]
fn a_name_cannot_pass_for_a_state() {
    assert!(runs_in("43 (a) Z 1 7 () R 1 42 42 0", group()));
    assert!(!runs_in("43 (a) R 1 42 () Z 1 42 42 0", group()));
}

#[test]
fn what_is_not_a_state_is_no_process() {
    for state in ["", "42", "42 (sh)", "42 (sh) S 1"] {
        assert!(!runs_in(state, group()), "{state:?}");
    }
}

#[test]
fn a_child_waiting_to_be_collected_is_not_alive() {
    let mut child = crate::launch::command("/bin/true").spawn().unwrap();
    let leader = child.id();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while crate::launch::group_is_alive(leader) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    assert!(!crate::launch::group_is_alive(leader));
    child.wait().unwrap();
}
