use std::ops::Range;

const NEW_LINE: char = '\n';
const SPACE: char = ' ';

pub(crate) fn wrap(text: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;

    for line in text.split(NEW_LINE) {
        wrap_line(line, line_start, width, &mut rows);
        line_start += line.len() + NEW_LINE.len_utf8();
    }
    rows
}

fn wrap_line(line: &str, offset: usize, width: usize, rows: &mut Vec<Range<usize>>) {
    let mut start = 0;
    loop {
        let rest = &line[start..];
        if columns(rest.trim_end_matches(SPACE)) <= width {
            rows.push(offset + start..offset + line.len());
            return;
        }

        let end = row_end(rest, width);
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

fn columns(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/wrap.rs"]
mod tests;
