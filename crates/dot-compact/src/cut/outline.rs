use crate::note::left_out;

const MOST_LINES: usize = 40;
const LONGEST_LINE: usize = 120;
const CUT_CHARACTERS: &str = "chars";
const OPENING_BRACE: &str = "{";
const HEADING_MARK: char = '#';
const HEADING_MARKS: std::ops::RangeInclusive<usize> = 2..=6;

#[derive(Clone, Copy)]
struct Written<'a> {
    number: usize,
    indent: usize,
    line: &'a str,
}

pub(super) fn of(lines: &str, first_number: usize, most_bytes: usize) -> Vec<String> {
    let written: Vec<Written<'_>> = lines
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(at, line)| Written {
            number: first_number + at,
            indent: line.len() - line.trim_start().len(),
            line,
        })
        .collect();
    let headings = headings(&written);
    let listed = if headings.is_empty() {
        shallowest_openers(&written)
    } else {
        headings
    };

    let mut room = most_bytes;
    listed
        .into_iter()
        .map(numbered)
        .take(MOST_LINES)
        .take_while(|line| {
            let fits = line.len() <= room;
            room = room.saturating_sub(line.len());
            fits
        })
        .collect()
}

fn headings<'a>(written: &[Written<'a>]) -> Vec<Written<'a>> {
    let follows_a_gap = |at: usize| {
        at.checked_sub(1)
            .is_none_or(|before| written[before].number + 1 < written[at].number)
    };
    written
        .iter()
        .enumerate()
        .filter(|(at, line)| is_a_heading(line.line) && follows_a_gap(*at))
        .map(|(_, line)| *line)
        .collect()
}

fn is_a_heading(line: &str) -> bool {
    let title = line.trim_start_matches(HEADING_MARK);
    let marks = line.len() - title.len();
    HEADING_MARKS.contains(&marks) && title.starts_with(' ')
}

fn shallowest_openers<'a>(written: &[Written<'a>]) -> Vec<Written<'a>> {
    let openers: Vec<Written<'a>> = written
        .windows(2)
        .filter(|pair| opens_a_block(pair[0], pair[1]))
        .map(|pair| pair[0])
        .collect();
    let shallowest = openers.iter().map(|opener| opener.indent).min();
    openers
        .into_iter()
        .filter(|opener| Some(opener.indent) == shallowest)
        .collect()
}

fn opens_a_block(line: Written<'_>, next: Written<'_>) -> bool {
    let named = line.line.contains(char::is_alphanumeric);
    let braced_below = next.indent == line.indent && next.line.trim() == OPENING_BRACE;
    named && (next.indent > line.indent || braced_below)
}

fn numbered(opener: Written<'_>) -> String {
    let line = opener.line.trim_end();
    let (kept, cut) = line.split_at(line.floor_char_boundary(LONGEST_LINE));
    if cut.is_empty() {
        return format!("{}:{kept}", opener.number);
    }
    format!(
        "{}:{kept}{}",
        opener.number,
        left_out(cut.chars().count(), CUT_CHARACTERS)
    )
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/cut/outline.rs"]
mod tests;
