use crate::report::rules::{INSTALL_PROGRESS, Lines, Rule, after_counter, starts_with_any};

const DEPRECATIONS: &str = "lines: deprecated packages";

const NPM_DEPRECATED: [&str; 2] = ["npm warn deprecated ", "npm WARN deprecated "];
const NPM_CHATTER: [&str; 5] = [
    "npm notice",
    "npm http ",
    "npm timing ",
    "npm verb ",
    "npm sill ",
];
const PNPM_DEPRECATED: [&str; 2] = ["WARN  deprecated ", "WARN deprecated "];
const PNPM_STEPS: [&str; 2] = ["Progress: resolved ", "Downloading "];
const YARN_FETCHES: [&str; 2] = ["➤ YN0013:", "➤ YN0019:"];
const PIP_STEPS: [&str; 14] = [
    "Attempting uninstall: ",
    "Building wheel for ",
    "Collecting ",
    "Created wheel for ",
    "Downloading ",
    "Found existing installation: ",
    "Getting requirements to build wheel",
    "Installing build dependencies",
    "Preparing metadata ",
    "Requirement already satisfied: ",
    "Stored in directory: ",
    "Successfully uninstalled ",
    "Uninstalling ",
    "Using cached ",
];
const UV_STEPS: [&str; 6] = [
    "Building ",
    "Built ",
    "Downloaded ",
    "Downloading ",
    "Preparing ",
    "Using cached ",
];
const MISE_STEPS: [&str; 3] = ["mise Downloading ", "mise Extracting ", "mise Installing "];
const PRE_COMMIT_SETUP: [&str; 4] = [
    "[INFO] Initializing environment",
    "[INFO] Installing environment",
    "[INFO] Once installed this environment will be reused",
    "[INFO] This may take a few minutes",
];
const POETRY_STEPS: [&str; 8] = [
    "- Downgrading ",
    "- Installing ",
    "- Removing ",
    "- Updating ",
    "• Downgrading ",
    "• Installing ",
    "• Removing ",
    "• Updating ",
];
const GEM_STEPS: [&str; 3] = ["Fetching ", "Installing ", "Using "];
const GEM_DOCUMENTATION: [&str; 3] = [
    "Done installing documentation for ",
    "Installing ri documentation for ",
    "Parsing documentation for ",
];
const COMPOSER_STEPS: [&str; 7] = [
    "- Downloading ",
    "- Installing ",
    "- Locking ",
    "- Removing ",
    "- Syncing ",
    "- Updating ",
    "- Upgrading ",
];
const POD_STEPS: [&str; 2] = ["Installing ", "Using "];
const BREW_STEPS: [&str; 4] = [
    "==> Downloading ",
    "==> Fetching ",
    "==> Pouring ",
    "Already downloaded: ",
];
const APT_STEPS: [&str; 12] = [
    "(Reading database ",
    "Building dependency tree",
    "Get:",
    "Hit:",
    "Ign:",
    "Preparing to unpack ",
    "Processing triggers for ",
    "Reading package lists",
    "Reading state information",
    "Selecting previously unselected package ",
    "Setting up ",
    "Unpacking ",
];
const APK_FETCH: &str = "fetch http";
const APK_STEPS: [&str; 3] = ["Installing ", "Purging ", "Upgrading "];
const GO_STEPS: [&str; 3] = ["go: downloading ", "go: extracting ", "go: finding "];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["npm", "npx"],
        what: DEPRECATIONS,
        lines: Lines::Starting(&NPM_DEPRECATED),
    },
    Rule {
        tools: &["npm", "npx"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&NPM_CHATTER),
    },
    Rule {
        tools: &["pnpm"],
        what: DEPRECATIONS,
        lines: Lines::Indented(&PNPM_DEPRECATED),
    },
    Rule {
        tools: &["pnpm"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&PNPM_STEPS),
    },
    Rule {
        tools: &["yarn"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&YARN_FETCHES),
    },
    Rule {
        tools: &["pip", "pip3"],
        what: INSTALL_PROGRESS,
        lines: Lines::Indented(&PIP_STEPS),
    },
    Rule {
        tools: &["uv"],
        what: INSTALL_PROGRESS,
        lines: Lines::Indented(&UV_STEPS),
    },
    Rule {
        tools: &["poetry"],
        what: INSTALL_PROGRESS,
        lines: Lines::Indented(&POETRY_STEPS),
    },
    Rule {
        tools: &["mise"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&MISE_STEPS),
    },
    Rule {
        tools: &["pre-commit"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&PRE_COMMIT_SETUP),
    },
    Rule {
        tools: &["bundle", "bundler"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&GEM_STEPS),
    },
    Rule {
        tools: &["gem"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&GEM_DOCUMENTATION),
    },
    Rule {
        tools: &["composer"],
        what: INSTALL_PROGRESS,
        lines: Lines::Indented(&COMPOSER_STEPS),
    },
    Rule {
        tools: &["pod"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&POD_STEPS),
    },
    Rule {
        tools: &["brew"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&BREW_STEPS),
    },
    Rule {
        tools: &["apt", "apt-get"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&APT_STEPS),
    },
    Rule {
        tools: &["apk"],
        what: INSTALL_PROGRESS,
        lines: Lines::Told(apk_step),
    },
    Rule {
        tools: &["go"],
        what: INSTALL_PROGRESS,
        lines: Lines::Starting(&GO_STEPS),
    },
];

fn apk_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line.starts_with(APK_FETCH)
        || after_counter(line, '(', ')').is_some_and(|step| starts_with_any(step, &APK_STEPS))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/rules/install.rs"]
mod tests;
