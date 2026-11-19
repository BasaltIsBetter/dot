use std::borrow::Cow;
use std::iter;
use std::ops::Range;

const NEW_LINE: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const TAB: char = '\t';
const SPACE: char = ' ';
const TAB_WIDTH: usize = 4;
const TAB_SHOWN: &str = "    ";
const MOST_BYTES_PER_CHARACTER: usize = 4;
const WINDOWS_NEW_LINE: &str = "\r\n";
const UNIX_NEW_LINE: &str = "\n";

type LineCutter = fn(&str, usize, usize, &mut Vec<Range<usize>>);

pub(crate) fn wrap(text: &str, width: usize) -> Vec<Range<usize>> {
    rows_of(text, width, wrap_line)
}

pub(crate) fn wrap_characters(text: &str, width: usize) -> Vec<Range<usize>> {
    rows_of(text, width, cut_line)
}

pub(crate) fn typed(text: &str, room: usize) -> String {
    text.replace(WINDOWS_NEW_LINE, UNIX_NEW_LINE)
        .chars()
        .map(|character| match character {
            CARRIAGE_RETURN => NEW_LINE,
            other => other,
        })
        .filter(|&character| !character.is_control() || matches!(character, TAB | NEW_LINE))
        .take(room)
        .collect()
}

pub(crate) const fn cells(character: char) -> usize {
    if character == TAB { TAB_WIDTH } else { 1 }
}

pub(crate) fn shown(text: &str) -> Cow<'_, str> {
    if text.contains(TAB) {
        return Cow::Owned(text.replace(TAB, TAB_SHOWN));
    }
    Cow::Borrowed(text)
}

pub(crate) fn normalized(text: &str, room: usize) -> String {
    text.replace(WINDOWS_NEW_LINE, UNIX_NEW_LINE)
        .chars()
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
        .collect()
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
    text[..probe].chars().count() <= width
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/wrap.rs"]
mod tests;
