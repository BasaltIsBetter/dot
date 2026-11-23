use crate::command::CommandLine;
use crate::note::{finish, push_line};
use crate::report::git::GIT;

const LOG: &str = "log";
const COMMIT: &str = "commit ";
const AUTHOR: &str = "Author: ";
const DATE: &str = "Date: ";
const SHORTEST_HASH: usize = 7;
const FEWEST_WORTH_REWRITING: usize = 2;

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    if !command.runs_with(GIT, LOG) || !text.lines().next().is_some_and(starts_a_commit) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let author = same_author(&lines)?;

    let mut shorter = String::with_capacity(text.len());
    push_line(
        &mut shorter,
        &format!("[git log: every commit by {author}]"),
    );
    for (at, line) in lines.iter().enumerate() {
        let told_once = line.starts_with(AUTHOR) || frames_a_message(&lines, at);
        if !told_once {
            push_line(&mut shorter, line);
        }
    }
    Some(finish(shorter, text))
}

fn same_author<'a>(lines: &[&'a str]) -> Option<&'a str> {
    let mut authors = lines.iter().filter_map(|line| line.strip_prefix(AUTHOR));
    let first = authors.next()?;
    let mut others = 0;
    for author in authors {
        if author != first {
            return None;
        }
        others += 1;
    }
    (others + 1 >= FEWEST_WORTH_REWRITING).then_some(first)
}

fn frames_a_message(lines: &[&str], at: usize) -> bool {
    let after_the_date = at > 0 && lines[at - 1].starts_with(DATE);
    let before_a_commit = lines.get(at + 1).is_some_and(|next| starts_a_commit(next));
    lines[at].is_empty() && (after_the_date || before_a_commit)
}

fn starts_a_commit(line: &str) -> bool {
    line.strip_prefix(COMMIT).is_some_and(|rest| {
        let hash = rest.split(' ').next().unwrap_or_default();
        hash.len() >= SHORTEST_HASH && hash.bytes().all(|digit| digit.is_ascii_hexdigit())
    })
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/git/log.rs"]
mod tests;
