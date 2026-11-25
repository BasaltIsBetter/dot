use std::fs;

use rustix::process::Pid;

const PROCESSES: &str = "/proc";
const STATE_FILE: &str = "stat";
const NAME_END: char = ')';
const ENDED: [&str; 2] = ["Z", "X"];

/// Returns whether a process of `group` has not ended yet, and is not only waiting to be
/// collected.
///
/// When `/proc` cannot be read, the group is taken for one that still runs.
pub(crate) fn is_more_than_ended_processes(group: Pid) -> bool {
    let Ok(entries) = fs::read_dir(PROCESSES) else {
        return true;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            name.as_encoded_bytes().iter().all(u8::is_ascii_digit)
        })
        .filter_map(|entry| fs::read_to_string(entry.path().join(STATE_FILE)).ok())
        .any(|state| runs_in(&state, group))
}

fn runs_in(state: &str, group: Pid) -> bool {
    let Some((_, fields)) = state.rsplit_once(NAME_END) else {
        return false;
    };
    let mut fields = fields.split_whitespace();
    let (state, _parent, led_by) = (fields.next(), fields.next(), fields.next());
    let runs = state.is_some_and(|state| !ENDED.contains(&state));
    runs && led_by.and_then(|led_by| led_by.parse().ok()) == Some(group.as_raw_pid())
}

#[cfg(test)]
#[path = "../../../tests/dot-process/ended.rs"]
mod tests;
