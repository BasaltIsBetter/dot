use crate::command::CommandLine;
use crate::note::{finish, push_line};
use crate::shape::{folder_note, shared_folder};

const TOOLS: [&str; 2] = ["du", "wc"];
const TOTAL: &str = "total";
const FEWEST_WORTH_REWRITING: usize = 4;
const LONGEST_UNIT: usize = 2;
const IN_A_NUMBER: [char; 2] = ['.', ','];

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    if !TOOLS.iter().any(|tool| command.mentions(tool)) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let columns = count_columns(&lines);
    let entries: Vec<(&str, &str)> = lines
        .iter()
        .map(|line| entry(line, columns))
        .collect::<Option<_>>()?;
    if entries.len() < FEWEST_WORTH_REWRITING {
        return None;
    }

    let paths = entries
        .iter()
        .map(|(_, path)| *path)
        .filter(|path| *path != TOTAL);
    let common = shared_folder(paths)?;
    let mut shorter = String::with_capacity(text.len());
    push_line(&mut shorter, &folder_note(common));
    for (counts, path) in entries {
        let counts: Vec<&str> = counts.split_whitespace().collect();
        let shown = path.strip_prefix(common).unwrap_or(path);
        push_line(&mut shorter, &format!("{} {shown}", counts.join(" ")));
    }
    Some(finish(shorter, text))
}

fn count_columns(lines: &[&str]) -> usize {
    let of_the_total = |line: &&str| {
        let mut fields: Vec<&str> = line.split_whitespace().collect();
        let counted = fields.pop() == Some(TOTAL) && fields.iter().all(|field| is_count(field));
        (counted && !fields.is_empty()).then_some(fields.len())
    };
    lines.iter().rev().find_map(of_the_total).unwrap_or(1)
}

fn entry(line: &str, columns: usize) -> Option<(&str, &str)> {
    let written = line.trim_start();
    let mut path = written;
    for _ in 0..columns {
        let end = path.find(char::is_whitespace)?;
        if !is_count(&path[..end]) {
            return None;
        }
        path = path[end..].trim_start();
    }
    let counts = &written[..written.len() - path.len()];
    (!path.is_empty()).then_some((counts, path))
}

fn is_count(field: &str) -> bool {
    let number = field.trim_end_matches(|unit: char| unit.is_ascii_alphabetic());
    let is_a_number = number.starts_with(|first: char| first.is_ascii_digit())
        && number
            .chars()
            .all(|digit| digit.is_ascii_digit() || IN_A_NUMBER.contains(&digit));
    is_a_number && field.len() - number.len() <= LONGEST_UNIT
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/table/sized.rs"]
mod tests;
