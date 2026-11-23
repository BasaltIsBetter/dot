use crate::report::rules::{BUILD_PROGRESS, Lines, Rule, contains_any, starts_with_any};

const IDLE_TASKS: &str = "lines: tasks with nothing to report";
const CACHE_STATUS: &str = "lines: cache status";
const IDLE_HOSTS: &str = "lines: hosts with nothing to report";

const TURBO_CACHE: [&str; 3] = ["cache bypass", "cache hit", "cache miss"];
const TURBO_TASK_CACHE: [&str; 3] = [": cache bypass", ": cache hit", ": cache miss"];
const TASK: &str = "task: Task ";
const UP_TO_DATE: &str = " is up to date";
const ANSIBLE_IDLE: [&str; 2] = ["ok: [", "skipping: ["];
const QUARTO_STEPS: [&str; 6] = [
    "Rendering",
    "Resolving",
    "Validating",
    "pandoc ",
    "processing file:",
    "running",
];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["turbo"],
        what: CACHE_STATUS,
        lines: Lines::Told(turbo_cache),
    },
    Rule {
        tools: &["task"],
        what: IDLE_TASKS,
        lines: Lines::Told(task_up_to_date),
    },
    Rule {
        tools: &["ansible", "ansible-playbook"],
        what: IDLE_HOSTS,
        lines: Lines::Starting(&ANSIBLE_IDLE),
    },
    Rule {
        tools: &["quarto"],
        what: BUILD_PROGRESS,
        lines: Lines::Indented(&QUARTO_STEPS),
    },
];

fn turbo_cache(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    starts_with_any(line.trim_start(), &TURBO_CACHE) || contains_any(line, &TURBO_TASK_CACHE)
}

fn task_up_to_date(lines: &[&str], at: usize) -> bool {
    lines[at].starts_with(TASK) && lines[at].contains(UP_TO_DATE)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/build/runner.rs"]
mod tests;
