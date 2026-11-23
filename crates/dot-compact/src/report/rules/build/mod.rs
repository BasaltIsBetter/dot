mod jvm;
mod native;
mod runner;

use crate::report::rules::{BUILD_PROGRESS, INSTALL_PROGRESS, Lines, Rule, starts_with_any};

pub(crate) use self::jvm::RULES as JVM;
pub(crate) use self::native::RULES as NATIVE;
pub(crate) use self::runner::RULES as RUNNERS;

const BENCHMARK_PROGRESS: &str = "lines: benchmark progress";

const CARGO_STEPS: [&str; 12] = [
    "Adding ",
    "Blocking ",
    "Checking ",
    "Compiling ",
    "Documenting ",
    "Downloaded ",
    "Downloading ",
    "Fresh ",
    "Locking ",
    "Packaging ",
    "Updating ",
    "Verifying ",
];
const DOTNET_RESTORE: [&str; 6] = [
    "All projects are up-to-date for restore",
    "Copyright (C) Microsoft",
    "Determining projects to restore",
    "Microsoft (R) ",
    "Restored ",
    "Restoring packages for ",
];
const NEXT_DONE: &str = "✓ ";
const NEXT_STEPS: [&str; 7] = [
    "Collecting build traces",
    "Collecting page data",
    "Compiled successfully",
    "Creating an optimized production build",
    "Finalizing page optimization",
    "Generating static pages",
    "Linting and checking validity of types",
];
const MIX_STEPS: [&str; 3] = ["==> ", "Compiling ", "Generated "];
const MIX_FETCHES: [&str; 4] = [
    "* Getting ",
    "Checking package (",
    "Fetched package",
    "Using locally cached package",
];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["cargo", "cross", "trunk"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(cargo_step),
    },
    Rule {
        tools: &["cargo"],
        what: BENCHMARK_PROGRESS,
        lines: Lines::Starting(&["Benchmarking "]),
    },
    Rule {
        tools: &["dotnet", "msbuild"],
        what: BUILD_PROGRESS,
        lines: Lines::Indented(&DOTNET_RESTORE),
    },
    Rule {
        tools: &["next"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(next_step),
    },
    Rule {
        tools: &["mix"],
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&MIX_STEPS),
    },
    Rule {
        tools: &["mix"],
        what: INSTALL_PROGRESS,
        lines: Lines::Indented(&MIX_FETCHES),
    },
];

fn cargo_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line.starts_with(' ') && starts_with_any(line.trim_start(), &CARGO_STEPS)
}

fn next_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_start();
    starts_with_any(line.strip_prefix(NEXT_DONE).unwrap_or(line), &NEXT_STEPS)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/build/mod.rs"]
mod tests;
