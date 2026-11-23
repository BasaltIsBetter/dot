use crate::report::rules::{INSTALL_PROGRESS, Lines, Rule, contains_any};

const STATE_REFRESHES: &str = "lines: state refreshes";
const RESOURCES_IN_PROGRESS: &str = "lines: resources in progress";
const LINKS: &str = "lines: links and advice";

const TERRAFORM_TOOLS: [&str; 3] = ["terraform", "terragrunt", "tofu"];
const TERRAFORM_WAITS: [&str; 6] = [
    ": Read complete after ",
    ": Reading...",
    ": Refreshing state...",
    ": Still creating...",
    ": Still destroying...",
    ": Still modifying...",
];
const TERRAFORM_SETUP: [&str; 11] = [
    "- Downloading ",
    "- Finding ",
    "- Installed ",
    "- Installing ",
    "- Reusing previous version ",
    "- Using previously-installed ",
    "Acquiring state lock",
    "Initializing modules",
    "Initializing provider plugins",
    "Initializing the backend",
    "Releasing state lock",
];
const PULUMI_LINKS: [&str; 7] = [
    "Duration:",
    "Loading policy packs",
    "More information at:",
    "Permalink:",
    "Use `pulumi ",
    "View Live:",
    "View in Browser",
];
const PULUMI_STEPS: [&str; 5] = [
    " creating (",
    " deleting (",
    " refreshing (",
    " refreshing",
    " updating (",
];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &TERRAFORM_TOOLS,
        what: STATE_REFRESHES,
        lines: Lines::Told(terraform_wait),
    },
    Rule {
        tools: &TERRAFORM_TOOLS,
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&TERRAFORM_SETUP),
    },
    Rule {
        tools: &["pulumi"],
        what: LINKS,
        lines: Lines::Starting(&PULUMI_LINKS),
    },
    Rule {
        tools: &["pulumi"],
        what: RESOURCES_IN_PROGRESS,
        lines: Lines::Told(pulumi_step),
    },
];

fn terraform_wait(lines: &[&str], at: usize) -> bool {
    contains_any(lines[at], &TERRAFORM_WAITS)
}

fn pulumi_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line.starts_with(' ') && contains_any(line.trim_end(), &PULUMI_STEPS)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/deploy/infra.rs"]
mod tests;
