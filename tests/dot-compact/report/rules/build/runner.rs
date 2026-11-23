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
fn what_turbo_found_in_its_cache() {
    let lines = [
        "web:build: cache hit, replaying logs 0123456789abcdef",
        "cache miss, executing 0123456789abcdef",
        "  cache bypass, force executing 0123",
        " Tasks:    3 successful, 3 total",
        "web:build: > next build",
        "a cache hit in a sentence",
    ];
    let cache = Some(CACHE_STATUS);

    assert_eq!(
        told("turbo", &lines),
        [cache, cache, cache, None, None, None]
    );
}

#[test]
fn a_task_that_is_up_to_date() {
    let lines = [
        "task: Task \"build\" is up to date",
        "task: [build] go build ./...",
        "task: Task \"test\" failed",
        "build is up to date",
    ];

    assert_eq!(told("task", &lines), [Some(IDLE_TASKS), None, None, None]);
}

#[test]
fn the_hosts_ansible_left_as_they_were() {
    let lines = [
        "ok: [web1]",
        "skipping: [web2]",
        "changed: [web3]",
        "fatal: [web4]: FAILED! => {\"msg\": \"no\"}",
        "PLAY RECAP *********",
        "TASK [install nginx] ***",
    ];
    let idle = Some(IDLE_HOSTS);

    assert_eq!(
        told("ansible-playbook", &lines),
        [idle, idle, None, None, None, None]
    );
}

#[test]
fn what_quarto_does_while_it_renders() {
    let lines = [
        "processing file: index.qmd",
        "  Validating schema",
        "pandoc --output index.html",
        "  running xelatex - 1",
        "Output created: index.html",
        "ERROR: YAML validation failed",
    ];

    assert_eq!(
        told("quarto", &lines),
        [
            Some(BUILD_PROGRESS),
            Some(BUILD_PROGRESS),
            Some(BUILD_PROGRESS),
            Some(BUILD_PROGRESS),
            None,
            None
        ]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "turbo",
            &["ok: [web1]", "task: Task \"build\" is up to date"]
        ),
        [None, None]
    );
}
