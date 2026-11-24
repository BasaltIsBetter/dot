use std::borrow::Cow;
use std::iter;
use std::ops::Range;

use crate::line::columns;

/// The character that ends a line.
pub const NEW_LINE: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const TAB: char = '\t';
const SPACE: char = ' ';
const TAB_WIDTH: usize = 4;
const TAB_SHOWN: &str = "    ";
const MOST_BYTES_PER_CHARACTER: usize = 4;

type LineCutter = fn(&str, usize, usize, &mut Vec<Range<usize>>);

/// Returns the rows of `text`, `width` columns wide, broken between words, as byte ranges.
///
/// After the last space that fits, the spaces at the end of a row go past it without taking
/// the word before down with them; a word longer than a row is cut; each line break starts a
/// row. Each row only counts the characters that can fit in it: linear time.
#[must_use]
pub fn wrap(text: &str, width: usize) -> Vec<Range<usize>> {
    rows_of(text, width, wrap_line)
}

/// Returns the rows of `text`, `width` cells wide, cut at the cell, spaces included, as byte
/// ranges: one more character never moves what comes before it. A tab takes 4 cells.
#[must_use]
pub fn wrap_characters(text: &str, width: usize) -> Vec<Range<usize>> {
    rows_of(text, width, cut_line)
}

/// Returns typed or pasted `text` cleaned for a draft, `room` characters at most: Windows line
/// ends become line breaks, control characters go, tabs stay.
#[must_use]
pub fn typed(text: &str, room: usize) -> String {
    unix_line_ends(text)
        .map(|character| match character {
            CARRIAGE_RETURN => NEW_LINE,
            other => other,
        })
        .filter(|&character| !character.is_control() || matches!(character, TAB | NEW_LINE))
        .take(room)
        .collect()
}

/// Returns how many cells `character` takes in a draft: 4 for a tab, 1 for anything else.
#[must_use]
pub const fn cells(character: char) -> usize {
    if character == TAB { TAB_WIDTH } else { 1 }
}

/// Returns `text` as a draft shows it: each tab drawn as 4 spaces.
#[must_use]
pub fn shown(text: &str) -> Cow<'_, str> {
    if text.contains(TAB) {
        return Cow::Owned(text.replace(TAB, TAB_SHOWN));
    }
    Cow::Borrowed(text)
}

/// Returns `text` cleaned for the history, `room` characters at most: Windows line ends become
/// line breaks, each tab 4 spaces, control characters go. Borrowed as it is when nothing in it
/// needs cleaning and it fits.
#[must_use]
pub fn normalized(text: &str, room: usize) -> Cow<'_, str> {
    let is_clean = text
        .chars()
        .all(|character| !character.is_control() || character == NEW_LINE);
    if is_clean && text.chars().nth(room).is_none() {
        return Cow::Borrowed(text);
    }
    let cleaned = unix_line_ends(text)
        .flat_map(|character| {
            let (kept, count) = match character {
                TAB => (SPACE, TAB_WIDTH),
                CARRIAGE_RETURN | NEW_LINE => (NEW_LINE, 1),
                control if control.is_control() => (control, 0),
                other => (other, 1),
            };
            iter::repeat_n(kept, count)
        })
        .take(room)
        .collect();
    Cow::Owned(cleaned)
}

fn unix_line_ends(text: &str) -> impl Iterator<Item = char> + '_ {
    let mut characters = text.chars().peekable();
    iter::from_fn(move || {
        let character = characters.next()?;
        if character == CARRIAGE_RETURN && characters.peek() == Some(&NEW_LINE) {
            return characters.next();
        }
        Some(character)
    })
}

fn rows_of(text: &str, width: usize, cut: LineCutter) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;

    for line in text.split(NEW_LINE) {
        cut(line, line_start, width, &mut rows);
        line_start += line.len() + NEW_LINE.len_utf8();
    }
    rows
}

fn cut_line(line: &str, offset: usize, width: usize, rows: &mut Vec<Range<usize>>) {
    if line.contains(TAB) {
        return cut_line_in_cells(line, offset, width, rows);
    }
    let mut start = 0;
    for (end, _) in line.char_indices().step_by(width).skip(1) {
        rows.push(offset + start..offset + end);
        start = end;
    }
    rows.push(offset + start..offset + line.len());
}

fn cut_line_in_cells(line: &str, offset: usize, width: usize, rows: &mut Vec<Range<usize>>) {
    let mut start = 0;
    let mut used = 0;
    for (index, character) in line.char_indices() {
        let wide = cells(character);
        if used > 0 && used + wide > width {
            rows.push(offset + start..offset + index);
            start = index;
            used = 0;
        }
        used += wide;
    }
    rows.push(offset + start..offset + line.len());
}

fn wrap_line(line: &str, offset: usize, width: usize, rows: &mut Vec<Range<usize>>) {
    let content_end = line.trim_end_matches(SPACE).len();
    let mut start = 0;
    loop {
        let content = line.get(start..content_end).unwrap_or_default();
        if fits(content, width) {
            rows.push(offset + start..offset + line.len());
            return;
        }

        let end = row_end(&line[start..], width);
        rows.push(offset + start..offset + start + end);
        start += end;
    }
}

fn row_end(rest: &str, width: usize) -> usize {
    let mut end = None;
    let mut after_space = false;

    for (column, (index, character)) in rest.char_indices().enumerate() {
        let is_space = character == SPACE;
        if after_space && !is_space {
            end = Some(index);
        }
        if !is_space && column >= width {
            break;
        }
        after_space = is_space;
    }
    end.unwrap_or_else(|| {
        rest.char_indices()
            .nth(width)
            .map_or(rest.len(), |(limit, _)| limit)
    })
}

fn fits(text: &str, width: usize) -> bool {
    let enough_for_one_more = width
        .saturating_add(1)
        .saturating_mul(MOST_BYTES_PER_CHARACTER);
    let probe = text.floor_char_boundary(enough_for_one_more);
    columns(&text[..probe]) <= width
}

#[cfg(test)]
#[path = "../../../tests/dot-transcript/wrap.rs"]
mod tests;
