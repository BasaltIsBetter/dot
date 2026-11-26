mod git;
mod marks;
mod progress;
mod rules;
mod stream;
mod table;

use crate::command::CommandLine;

pub use self::stream::LineStream;

type Stage = fn(&CommandLine<'_>, &str) -> Option<String>;

pub(crate) fn rewrite(command: &CommandLine<'_>, text: &str) -> Option<String> {
    run(
        &[
            git::status,
            git::log,
            table::listing,
            table::query,
            table::sized,
        ],
        command,
        text,
    )
}

pub(crate) fn count(command: &CommandLine<'_>, text: &str) -> Option<String> {
    run(&[marks::compact, progress::compact], command, text)
}

fn run(stages: &[Stage], command: &CommandLine<'_>, text: &str) -> Option<String> {
    let mut shorter: Option<String> = None;
    for stage in stages {
        let current = shorter.as_deref().unwrap_or(text);
        if let Some(next) = stage(command, current) {
            shorter = Some(next);
        }
    }
    shorter
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/report/mod.rs"]
mod tests;
