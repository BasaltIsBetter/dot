use crate::command::CommandLine;
use crate::note::{finish, left_out, push_line};

const TOOLS: [&str; 4] = ["mix", "rake", "rspec", "rubocop"];
const PASSED: char = '.';
const PENDING: char = '*';
const FEWEST_MARKS: usize = 20;
const PASSED_IN_FOUR: usize = 3;

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    if !TOOLS.iter().any(|tool| command.mentions(tool)) {
        return None;
    }

    let mut shorter = String::with_capacity(text.len());
    let mut changed = false;
    for line in text.lines() {
        match tally(line.trim()) {
            Some(counted) => {
                push_line(&mut shorter, &counted);
                changed = true;
            }
            None => push_line(&mut shorter, line),
        }
    }
    changed.then(|| finish(shorter, text))
}

fn tally(line: &str) -> Option<String> {
    let mut marks: Vec<(char, usize)> = Vec::new();
    let mut total = 0;
    for mark in line.chars() {
        let is_a_mark = mark == PASSED || mark == PENDING || mark.is_ascii_uppercase();
        if !is_a_mark {
            return None;
        }
        match marks.iter_mut().find(|(known, _)| *known == mark) {
            Some((_, count)) => *count += 1,
            None => marks.push((mark, 1)),
        }
        total += 1;
    }
    let passed = marks
        .iter()
        .find(|(mark, _)| *mark == PASSED)
        .map_or(0, |(_, count)| *count);
    if total < FEWEST_MARKS || passed * 4 < total * PASSED_IN_FOUR {
        return None;
    }

    let counted: Vec<String> = marks
        .iter()
        .map(|(mark, count)| format!("{count} {mark}"))
        .collect();
    Some(left_out(total, &format!("marks: {}", counted.join(" "))))
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/report/marks.rs"]
mod tests;
