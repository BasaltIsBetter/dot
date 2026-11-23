use crate::cut::thin::thin;
use crate::note::{finish, left_out, push_line};
use crate::shape;

const FEWEST_LEFT_OUT: usize = 3;
const UNCHANGED_LINES: &str = "unchanged lines";
const LINES_OF_MESSAGE: &str = "lines of message";
const CONTEXT: char = ' ';
const CHANGES: [char; 2] = ['+', '-'];
const FILE_HEADERS: [&str; 2] = ["+++ ", "--- "];
const COMMIT: &str = "commit ";
const MESSAGE: &str = "    ";
const FEWEST_COMMITS: usize = 2;
const HASH_LENGTH: usize = 40;
const SHORT_HASH: usize = 7;
const AUTHOR: &str = "Author: ";
const DATE: &str = "Date:";
const MERGE: &str = "Merge: ";
const ONE_LINE_PER_COMMIT: &str = "[git log as: hash day subject, one line per commit]";

struct Commit<'a> {
    hash: &'a str,
    names: &'a str,
    author: Option<&'a str>,
    day: Option<String>,
    subject: Option<&'a str>,
    lines_below: usize,
}

impl<'a> Commit<'a> {
    fn opened_by(line: &'a str) -> Option<Self> {
        let (hash, names) = line.strip_prefix(COMMIT)?.split_at_checked(HASH_LENGTH)?;
        let is_a_hash = hash.bytes().all(|byte| byte.is_ascii_hexdigit());
        let is_alone = names.is_empty() || names.starts_with(' ');
        (is_a_hash && is_alone).then_some(Self {
            hash,
            names,
            author: None,
            day: None,
            subject: None,
            lines_below: 0,
        })
    }

    fn read(&mut self, line: &'a str) -> Option<()> {
        if let Some(author) = line.strip_prefix(AUTHOR) {
            self.author = Some(author.split(" <").next().unwrap_or(author));
        } else if let Some(date) = line.strip_prefix(DATE) {
            self.day = Some(day_of(date));
        } else if let Some(message) = line.strip_prefix(MESSAGE) {
            match self.subject {
                None => self.subject = Some(message),
                Some(_) => self.lines_below += 1,
            }
        } else if !line.trim().is_empty() && !line.starts_with(MERGE) {
            return None;
        }
        Some(())
    }

    fn written(&self) -> Option<String> {
        let author = self
            .author
            .map_or_else(String::new, |author| format!(" <{author}>"));
        let below = match self.lines_below {
            0 => String::new(),
            lines => format!(" [+{lines} lines]"),
        };
        Some(format!(
            "{}{} {}{author} {}{below}",
            &self.hash[..SHORT_HASH],
            self.names,
            self.day.as_deref()?,
            self.subject?
        ))
    }
}

pub(crate) fn far_context(text: &str) -> Option<String> {
    if !shape::is_diff(text) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let is_far = |at: usize| {
        let near = |other: Option<&&str>| other.is_some_and(|line| is_changed(line));
        let before = at.checked_sub(1).and_then(|before| lines.get(before));
        lines[at].starts_with(CONTEXT) && !near(before) && !near(lines.get(at + 1))
    };
    leaving_out(text, &lines, is_far, UNCHANGED_LINES)
}

pub(crate) fn commits(text: &str) -> Option<String> {
    if !text.contains(COMMIT) {
        return None;
    }
    let mut thinner = String::with_capacity(text.len() / 2);
    let mut commits: Vec<Commit<'_>> = Vec::new();
    for line in text.lines() {
        if let Some(commit) = Commit::opened_by(line) {
            commits.push(commit);
        } else if let Some(commit) = commits.last_mut() {
            commit.read(line)?;
        } else {
            push_line(&mut thinner, line);
        }
    }
    if commits.len() < FEWEST_COMMITS {
        return None;
    }

    push_line(&mut thinner, ONE_LINE_PER_COMMIT);
    for commit in &commits {
        push_line(&mut thinner, &commit.written()?);
    }
    Some(finish(thinner, text))
}

fn day_of(date: &str) -> String {
    let fields: Vec<&str> = date.split_whitespace().collect();
    match fields.as_slice() {
        [_, month, day, _, year, ..] => format!("{month} {day} {year}"),
        _ => date.trim().to_owned(),
    }
}

pub(crate) fn messages(text: &str) -> Option<String> {
    if !text.contains(COMMIT) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let commits = lines.iter().filter(|line| line.starts_with(COMMIT)).count();
    if commits < FEWEST_COMMITS {
        return None;
    }
    let below_the_subject = below_the_subject(&lines);
    leaving_out(text, &lines, |at| below_the_subject[at], LINES_OF_MESSAGE)
}

fn below_the_subject(lines: &[&str]) -> Vec<bool> {
    let mut follows_a_line_of_message = false;
    let mut follows_a_message = false;
    lines
        .iter()
        .map(|line| {
            let is_empty = line.trim().is_empty();
            let is_of_a_message = line.starts_with(MESSAGE);
            let below = ((is_of_a_message || is_empty) && follows_a_line_of_message)
                || (is_empty && follows_a_message);

            follows_a_line_of_message = is_of_a_message;
            if !is_empty {
                follows_a_message = is_of_a_message;
            }
            below
        })
        .collect()
}

fn is_changed(line: &str) -> bool {
    line.starts_with(CHANGES) && !FILE_HEADERS.iter().any(|header| line.starts_with(header))
}

fn leaving_out(
    text: &str,
    lines: &[&str],
    is_left_out: impl Fn(usize) -> bool,
    what: &str,
) -> Option<String> {
    let run_at = |at: usize| {
        (at..lines.len())
            .take_while(|&line| is_left_out(line))
            .count()
    };
    thin(text, lines, FEWEST_LEFT_OUT, run_at, |thinner, run| {
        push_line(thinner, &left_out(run.len(), what));
    })
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/cut/thin/git.rs"]
mod tests;
