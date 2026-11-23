mod git;
mod marks;
mod progress;
mod rules;
mod stream;
mod table;

use crate::command::CommandLine;

pub use self::stream::LineStream;

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    let stages = [
        git::status,
        git::log,
        table::listing,
        table::query,
        table::sized,
        marks::compact,
        progress::compact,
    ];
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
