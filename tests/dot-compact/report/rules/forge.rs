use super::*;

fn told(tool: &str, lines: &[&str]) -> Vec<Option<&'static str>> {
    (0..lines.len())
        .map(|at| {
            RULES
                .iter()
                .filter(|rule| rule.tools.contains(&tool))
                .find(|rule| rule.drops(lines, at))
                .map(|rule| rule.what)
        })
        .collect()
}

#[test]
fn the_checks_of_a_pull_request_that_passed() {
    let lines = [
        "CodeQL\tpass\t2s\thttps://github.com/cli/cli/runs/135772291299\t",
        "build (macos-latest)\tpass\t5m25s\thttps://github.com/cli/cli/actions/runs/44584003105/job/135771914833\t",
        "lint\tfail\t1m2s\thttps://github.com/cli/cli/actions/runs/1/job/2\t",
        "deploy\tpending\t0\thttps://github.com/cli/cli/actions/runs/1/job/3\t",
        "docs\tskipping\t0\thttps://github.com/cli/cli/actions/runs/1/job/4\t",
        "pass\tfail\t3s\thttps://example.com\t",
        "\tpass\t3s",
        "name\tpass",
        "the tests pass",
    ];
    let passed = Some(PASSED_CHECKS);

    assert_eq!(
        told("gh", &lines),
        [passed, passed, None, None, None, None, None, None, None]
    );
}

#[test]
fn what_a_gitlab_runner_says_before_the_job() {
    let lines = [
        "Running with gitlab-runner 16.11.0 (91a27b2a)",
        "  on runner-abc123 xyz, system ID: s_1234",
        "Using Docker executor with image node:20 ...",
        "Pulling docker image node:20 ...",
        "Fetching changes with git depth set to 20...",
        "Checking out 1a2b3c4d as detached HEAD (ref is main)...",
        "$ npm ci",
        "on the other hand, nothing",
        "npm error Running with gitlab-runner",
    ];
    let setup = Some(RUNNER_SETUP);

    assert_eq!(
        told("glab", &lines),
        [setup, setup, setup, setup, setup, setup, None, None, None]
    );
}

#[test]
fn the_branches_graphite_restacked() {
    let lines = [
        "Restacked branch feat/add-auth on main",
        "Pushed branch feat/add-auth to origin",
        "  Restacked branch in a sentence",
    ];

    assert_eq!(told("gt", &lines), [Some(RESTACKED), None, None]);
}

#[test]
fn a_tool_only_has_its_own_rules() {
    let lines = [
        "CodeQL\tpass\t2s\thttps://github.com/cli/cli/runs/1\t",
        "Running with gitlab-runner 16.11.0 (91a27b2a)",
        "Restacked branch feat/add-auth on main",
    ];

    assert_eq!(told("git", &lines), [None; 3]);
    assert_eq!(told("gh", &lines), [Some(PASSED_CHECKS), None, None]);
    assert_eq!(told("glab", &lines), [None, Some(RUNNER_SETUP), None]);
    assert_eq!(told("gt", &lines), [None, None, Some(RESTACKED)]);
}
