use crate::report::rules::{Lines, Rule};

const NOTICES: &str = "lines: notices";
const UNCHANGED_FILES: &str = "lines: files left unchanged";
const RULE_LINKS: &str = "lines: links to the rules";
const LEGEND: &str = "lines: legend";

const PRETTIER_UNCHANGED: &str = " (unchanged)";
const SHELLCHECK_LINKS: &str = "For more information:";
const SHELLCHECK_WIKI: &str = "https://www.shellcheck.net/wiki/";
const PRISMA_NOTICES: [&str; 7] = [
    "Help us improve the Prisma ORM",
    "Start by importing your Prisma Client",
    "Start using Prisma Client",
    "Tip: ",
    "┌",
    "│",
    "└",
];
const SYSTEMD_LEGEND: &str = "Legend: ";
const SYSTEMD_STATES: [&str; 3] = ["ACTIVE", "LOAD", "SUB"];
const SYSTEMD_MEANS: [char; 2] = ['→', '='];
const GCLOUD_NOTICES: [&str; 6] = [
    "  $ gcloud components update",
    "  $ gcloud survey",
    "To take a quick anonymous survey, run:",
    "Updates are available for some Cloud SDK components",
    "Updates are available for some Google Cloud CLI components",
    "please run:",
];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["prettier"],
        what: UNCHANGED_FILES,
        lines: Lines::Ending(&[PRETTIER_UNCHANGED]),
    },
    Rule {
        tools: &["shellcheck"],
        what: RULE_LINKS,
        lines: Lines::Told(shellcheck_link),
    },
    Rule {
        tools: &["prisma"],
        what: NOTICES,
        lines: Lines::Starting(&PRISMA_NOTICES),
    },
    Rule {
        tools: &["systemctl"],
        what: LEGEND,
        lines: Lines::Told(systemd_legend),
    },
    Rule {
        tools: &["gcloud"],
        what: NOTICES,
        lines: Lines::Starting(&GCLOUD_NOTICES),
    },
];

fn shellcheck_link(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line == SHELLCHECK_LINKS || line.trim_start().starts_with(SHELLCHECK_WIKI)
}

fn systemd_legend(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_start();
    let line = line.strip_prefix(SYSTEMD_LEGEND).unwrap_or(line);
    SYSTEMD_STATES.iter().any(|state| {
        line.strip_prefix(state)
            .is_some_and(|meaning| meaning.trim_start().starts_with(SYSTEMD_MEANS))
    })
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/rules/notice.rs"]
mod tests;
