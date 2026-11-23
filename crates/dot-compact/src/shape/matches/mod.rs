mod headed;
mod hit;

use std::collections::BTreeSet;

use crate::note::{NEW_LINE, finish, left_out, push_line};
use crate::shape::prefix::{common_folder, note};

pub(crate) use self::headed::name_folder;
use self::hit::{Hit, POSITION_END, context, numbered, positioned};

const CONTEXT_BREAK: &str = "--";
const FEWEST_WORTH_GROUPING: usize = 4;
const LONGEST_TEXT: usize = 400;
const CUT_CHARACTERS: &str = "chars";

pub(crate) fn group(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let gives_positions = text.contains(POSITION_END);
    let matched =
        |line| numbered(line).or_else(|| gives_positions.then(|| positioned(line)).flatten());
    let files: BTreeSet<&str> = lines
        .iter()
        .filter_map(|line| matched(line))
        .map(|hit| hit.path)
        .collect();
    let hits: Vec<Option<Hit<'_>>> = lines
        .iter()
        .map(|line| matched(line).or_else(|| context(line, &files)))
        .collect();
    if hits.iter().flatten().count() < FEWEST_WORTH_GROUPING {
        return None;
    }

    let common = common_folder(files.iter().copied()).unwrap_or_default();
    let mut grouped = String::with_capacity(text.len());
    if !common.is_empty() {
        push_line(&mut grouped, &note(common));
    }
    let mut at = 0;
    while at < lines.len() {
        let Some(hit) = hits[at] else {
            push_line(&mut grouped, lines[at]);
            at += 1;
            continue;
        };
        let run = run_of(hit.path, &lines[at..], &hits[at..]);
        push_run(
            &mut grouped,
            hit.path,
            common,
            &lines[at..at + run],
            &hits[at..at + run],
        );
        at += run;
    }

    let grouped = finish(grouped, text);
    (grouped.len() < text.len()).then_some(grouped)
}

fn run_of(path: &str, lines: &[&str], hits: &[Option<Hit<'_>>]) -> usize {
    let same_file = |at: usize| {
        hits.get(at)
            .copied()
            .flatten()
            .is_some_and(|hit| hit.path == path)
    };
    let mut run = 0;
    loop {
        if same_file(run) {
            run += 1;
        } else if lines.get(run) == Some(&CONTEXT_BREAK) && same_file(run + 1) {
            run += 2;
        } else {
            let shown = shown_under(lines.get(run..).unwrap_or_default());
            if shown == 0 || !same_file(run + shown) {
                return run;
            }
            run += shown;
        }
    }
}

fn shown_under(lines: &[&str]) -> usize {
    lines
        .iter()
        .take_while(|line| line.starts_with(char::is_whitespace))
        .count()
}

fn push_run(
    grouped: &mut String,
    path: &str,
    common: &str,
    lines: &[&str],
    hits: &[Option<Hit<'_>>],
) {
    if let ([only], [hit]) = (lines, hits) {
        let whole = hit.is_some_and(|hit| hit.whole);
        return push_row(grouped, only.strip_prefix(common).unwrap_or(only), whole);
    }
    let shown = path.strip_prefix(common).unwrap_or(path);
    let follows_text = !grouped.is_empty() && !grouped.ends_with("\n\n");
    if follows_text {
        grouped.push(NEW_LINE);
    }
    push_line(grouped, shown);
    for (line, hit) in lines.iter().zip(hits) {
        match hit {
            Some(hit) => push_row(grouped, hit.row, hit.whole),
            None => push_line(grouped, line),
        }
    }
}

fn push_row(grouped: &mut String, row: &str, whole: bool) {
    let (kept, cut) = row.split_at(cut_at(row));
    if whole || cut.is_empty() {
        return push_line(grouped, row);
    }
    grouped.push_str(kept);
    push_line(grouped, &left_out(cut.chars().count(), CUT_CHARACTERS));
}

fn cut_at(row: &str) -> usize {
    if row.len() <= LONGEST_TEXT {
        return row.len();
    }
    (0..=LONGEST_TEXT)
        .rev()
        .find(|&at| row.is_char_boundary(at))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/shape/matches/mod.rs"]
mod tests;
