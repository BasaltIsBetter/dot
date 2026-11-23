use crate::report::rules::{Lines, Rule, starts_with_any};

const PASSED_CHECKS: &str = "lines: checks that passed";
const RUNNER_SETUP: &str = "lines: runner setup";
const RESTACKED: &str = "lines: branches restacked";

const FIELD_BREAK: char = '\t';
const CHECK_PASSED: &str = "pass";
const GITLAB_RUNNER: [&str; 17] = [
    "Checking out ",
    "Created fresh repository",
    "Fetching changes with git",
    "Getting source from",
    "Initialized empty Git",
    "Preparing environment",
    "Preparing the",
    "Pulling docker image",
    "Resolving secrets",
    "Running on runner-",
    "Running with gitlab-runner",
    "Runtime platform",
    "Skipping Git submodules",
    "Using Docker executor",
    "Using Kubernetes",
    "Using Shell",
    "Using docker image",
];
const GITLAB_HOST: &str = "on ";
const GITLAB_SYSTEM: &str = "system ID:";

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["gh"],
        what: PASSED_CHECKS,
        lines: Lines::Told(check_passed),
    },
    Rule {
        tools: &["glab"],
        what: RUNNER_SETUP,
        lines: Lines::Told(runner_setup),
    },
    Rule {
        tools: &["gt"],
        what: RESTACKED,
        lines: Lines::Starting(&["Restacked branch "]),
    },
];

fn check_passed(lines: &[&str], at: usize) -> bool {
    let mut fields = lines[at].split(FIELD_BREAK);
    let named = fields.next().is_some_and(|name| !name.is_empty());
    named && fields.next() == Some(CHECK_PASSED) && fields.next().is_some()
}

fn runner_setup(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_start();
    starts_with_any(line, &GITLAB_RUNNER)
        || (line.starts_with(GITLAB_HOST) && line.contains(GITLAB_SYSTEM))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/rules/forge.rs"]
mod tests;
