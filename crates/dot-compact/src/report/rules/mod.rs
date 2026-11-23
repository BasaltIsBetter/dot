mod build;
mod deploy;
mod forge;
mod install;
mod notice;
mod passed;

pub(crate) const BUILD_PROGRESS: &str = "lines: build progress";
pub(crate) const INSTALL_PROGRESS: &str = "lines: install progress";
pub(crate) const TRANSFER_PROGRESS: &str = "lines: transfer progress";
pub(crate) const PASSED_TESTS: &str = "lines: tests that passed";

pub(crate) const RULES: [&[Rule]; 11] = [
    build::RULES,
    build::JVM,
    build::NATIVE,
    build::RUNNERS,
    install::RULES,
    passed::RULES,
    deploy::RULES,
    deploy::CONTAINERS,
    deploy::INFRASTRUCTURE,
    forge::RULES,
    notice::RULES,
];

type Told = fn(&[&str], usize) -> bool;

pub(crate) struct Rule {
    pub(crate) tools: &'static [&'static str],
    pub(crate) what: &'static str,
    pub(crate) lines: Lines,
}

pub(crate) enum Lines {
    Starting(&'static [&'static str]),
    Indented(&'static [&'static str]),
    Ending(&'static [&'static str]),
    Told(Told),
}

impl Rule {
    pub(crate) fn drops(&self, lines: &[&str], at: usize) -> bool {
        let line = lines[at];
        match self.lines {
            Lines::Starting(starts) => starts_with_any(line, starts),
            Lines::Indented(starts) => starts_with_any(line.trim_start(), starts),
            Lines::Ending(ends) => ends.iter().any(|end| line.trim_end().ends_with(end)),
            Lines::Told(told) => told(lines, at),
        }
    }
}

pub(crate) fn starts_with_any(line: &str, starts: &[&str]) -> bool {
    starts.iter().any(|start| line.starts_with(start))
}

pub(crate) fn contains_any(line: &str, parts: &[&str]) -> bool {
    parts.iter().any(|part| line.contains(part))
}

pub(crate) fn after_counter(line: &str, open: char, close: char) -> Option<&str> {
    let (counter, rest) = line.strip_prefix(open)?.split_once(close)?;
    let counter = counter.trim();
    let counts = counter.starts_with(|first: char| first.is_ascii_digit())
        && counter
            .chars()
            .all(|digit| digit.is_ascii_digit() || digit == '/' || digit == '%');
    counts.then(|| rest.trim_start())
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/rules/mod.rs"]
mod tests;
