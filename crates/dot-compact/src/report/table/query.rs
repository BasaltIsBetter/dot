use crate::command::CommandLine;
use crate::note::{finish, push_line};

const PSQL: &str = "psql";
const CELL_BREAK: char = '|';
const RULE: char = '-';
const RULE_BREAK: char = '+';

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    if !command.mentions(PSQL) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();

    let mut shorter = String::with_capacity(text.len());
    let mut changed = false;
    let mut at = 0;
    while at < lines.len() {
        if let Some((rows, length)) = table_at(&lines[at..]) {
            shorter.push_str(&rows);
            changed = true;
            at += length;
        } else {
            push_line(&mut shorter, lines[at]);
            at += 1;
        }
    }
    changed.then(|| finish(shorter, text))
}

fn table_at(lines: &[&str]) -> Option<(String, usize)> {
    let breaks = breaks_of(lines.get(1)?)?;
    let below = lines[2..]
        .iter()
        .take_while(|line| line.contains(CELL_BREAK))
        .count();

    let mut rows = String::new();
    let written = std::iter::once(&lines[0]).chain(&lines[2..2 + below]);
    for line in written {
        push_line(&mut rows, &cells(line, &breaks)?.join("|"));
    }
    Some((rows, 2 + below))
}

fn breaks_of(rule: &str) -> Option<Vec<usize>> {
    let is_a_rule = rule.starts_with(RULE)
        && rule
            .chars()
            .all(|drawn| drawn == RULE || drawn == RULE_BREAK);
    let breaks: Vec<usize> = rule
        .chars()
        .enumerate()
        .filter_map(|(column, drawn)| (drawn == RULE_BREAK).then_some(column))
        .collect();
    (is_a_rule && !breaks.is_empty()).then_some(breaks)
}

fn cells<'a>(line: &'a str, breaks: &[usize]) -> Option<Vec<&'a str>> {
    let mut cells = Vec::with_capacity(breaks.len() + 1);
    let mut start = 0;
    let mut wanted = breaks.iter().copied().peekable();
    for (column, (offset, character)) in line.char_indices().enumerate() {
        if wanted.next_if_eq(&column).is_none() {
            continue;
        }
        if character != CELL_BREAK {
            return None;
        }
        cells.push(line[start..offset].trim());
        start = offset + CELL_BREAK.len_utf8();
    }
    cells.push(line[start..].trim());

    let whole = wanted.peek().is_none() && cells.iter().all(|cell| !cell.contains(CELL_BREAK));
    whole.then_some(cells)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/table/query.rs"]
mod tests;
