use crate::note::{finish, push_line};
use crate::shape::matches::hit::{FIELD, is_number};
use crate::shape::prefix::{common_folder, note};

const FOLDER: char = '/';
const LINE_OF_A_PATH: &str = "\n/";

pub(crate) fn name_folder(text: &str) -> Option<String> {
    if !text.contains(LINE_OF_A_PATH) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let heads: Vec<bool> = (0..lines.len())
        .map(|at| heads_findings(&lines, at))
        .collect();
    let files = lines
        .iter()
        .zip(&heads)
        .filter_map(|(line, head)| head.then_some(*line));
    let common = common_folder(files)?;

    let mut named = String::with_capacity(text.len());
    push_line(&mut named, &note(common));
    for (line, head) in lines.iter().zip(&heads) {
        let shown = if *head {
            line.strip_prefix(common).unwrap_or(line)
        } else {
            line
        };
        push_line(&mut named, shown);
    }
    Some(finish(named, text))
}

fn heads_findings(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let is_a_path = line.starts_with(FOLDER) && !line.contains(char::is_whitespace);
    is_a_path && lines.get(at + 1).is_some_and(|next| is_finding(next))
}

fn is_finding(line: &str) -> bool {
    let position = line
        .split_whitespace()
        .next()
        .and_then(|word| word.split_once(FIELD));
    line.starts_with(char::is_whitespace)
        && position.is_some_and(|(row, column)| is_number(row) && is_number(column))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/shape/matches/headed.rs"]
mod tests;
