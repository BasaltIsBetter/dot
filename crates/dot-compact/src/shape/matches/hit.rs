use std::collections::BTreeSet;

pub(super) const FIELD: char = ':';
pub(super) const CONTEXT: char = '-';

const WORD_BREAK: char = ' ';
const POSITION_START: char = '(';
pub(super) const POSITION_END: &str = "): ";
const POSITION_BREAK: char = ',';
const MOST_POSITION_NUMBERS: usize = 4;
const CODE_LETTERS: std::ops::RangeInclusive<usize> = 2..=4;
const FEWEST_CODE_DIGITS: usize = 3;
const CLOCK_DIGITS: usize = 2;

#[derive(Clone, Copy)]
pub(super) struct Hit<'a> {
    pub(super) path: &'a str,
    pub(super) row: &'a str,
    pub(super) whole: bool,
}

pub(super) fn context<'a>(line: &'a str, files: &BTreeSet<&str>) -> Option<Hit<'a>> {
    line.match_indices(CONTEXT).find_map(|(at, _)| {
        let (path, row) = (&line[..at], &line[at + 1..]);
        let (number, _) = row.split_once(CONTEXT)?;
        (is_number(number) && files.contains(path)).then_some(Hit {
            path,
            row,
            whole: false,
        })
    })
}

pub(super) fn numbered(line: &str) -> Option<Hit<'_>> {
    let (path, row) = line.split_once(FIELD)?;
    let searched = row
        .split_once(FIELD)
        .is_some_and(|(number, _)| is_number(number));
    let coded = || {
        row.split_once(WORD_BREAK)
            .is_some_and(|(number, rest)| is_number(number) && starts_with_a_rule_code(rest))
    };
    ((searched || coded()) && is_path(path) && !is_a_clock(path, row)).then_some(Hit {
        path,
        row,
        whole: !searched,
    })
}

pub(super) fn positioned(line: &str) -> Option<Hit<'_>> {
    let open = line.find(POSITION_START)?;
    let (path, row) = line.split_at(open);
    let (position, _) = row[POSITION_START.len_utf8()..].split_once(POSITION_END)?;
    let numbers = position.split(POSITION_BREAK).count();
    let is_a_position =
        numbers <= MOST_POSITION_NUMBERS && position.split(POSITION_BREAK).all(is_number);
    (is_path(path) && is_a_position).then_some(Hit {
        path,
        row,
        whole: true,
    })
}

fn is_a_clock(before: &str, row: &str) -> bool {
    let hours = before.bytes().rev().take_while(u8::is_ascii_digit).count();
    hours == CLOCK_DIGITS
        && row.split_once(FIELD).is_some_and(|(minutes, after)| {
            let seconds = after.bytes().take_while(u8::is_ascii_digit).count();
            minutes.len() == CLOCK_DIGITS && seconds == CLOCK_DIGITS
        })
}

fn is_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(char::is_whitespace)
        && !path.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn is_number(field: &str) -> bool {
    !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit())
}

fn starts_with_a_rule_code(text: &str) -> bool {
    let letters = text.bytes().take_while(u8::is_ascii_uppercase).count();
    let digits = text[letters..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    let ends_the_word = text[letters + digits..]
        .chars()
        .next()
        .is_none_or(|next| !next.is_alphanumeric());
    CODE_LETTERS.contains(&letters) && digits >= FEWEST_CODE_DIGITS && ends_the_word
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/shape/matches/hit.rs"]
mod tests;
