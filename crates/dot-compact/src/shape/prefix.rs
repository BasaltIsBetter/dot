use crate::note::{NEW_LINE, finish, push_line};

const FOLDER: char = '/';
const SHORTEST_WORTH_NAMING: usize = 16;
const FEWEST_NEIGHBOURS: usize = 3;
const FOLDER_BYTE: u8 = b'/';
const ENDS_A_PATH: u8 = b':';
const UNDER_THE_NOTE: char = ' ';

struct Neighbours<'a> {
    lines: usize,
    shared: &'a str,
    length: usize,
}

pub(crate) fn common_folder<'a>(paths: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut paths = paths.into_iter().peekable();
    let is_absolute = paths.peek()?.starts_with(FOLDER);
    is_absolute.then(|| shared_folder(paths)).flatten()
}

pub(crate) fn shared_folder<'a>(paths: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut paths = paths.into_iter();
    let first = paths.next()?;

    let mut common = folder_of(first);
    let mut others = 0_usize;
    for path in paths {
        while !path.starts_with(common) {
            common = folder_of(common.strip_suffix(FOLDER)?);
        }
        others += 1;
    }
    (others > 0 && common.len() >= SHORTEST_WORTH_NAMING).then_some(common)
}

pub(crate) fn name_neighbours(text: &str) -> Option<String> {
    let mut named: Option<String> = None;
    let mut copied = 0;
    let mut at = 0;

    while let Some(slash) = text[at..].find(FOLDER) {
        at = line_start(&text[..at + slash]);
        let run = neighbours(&text[at..]);
        if run.lines < FEWEST_NEIGHBOURS {
            at += line_end(&text[at..]);
            continue;
        }
        let block = at..at + run.length;
        at = block.end;
        if starts_indented(&text[at..]) {
            continue;
        }
        let written = named.get_or_insert_with(|| String::with_capacity(text.len()));
        written.push_str(&text[copied..block.start]);
        push_line(written, &note(run.shared));
        for line in text[block].lines() {
            written.push(UNDER_THE_NOTE);
            push_line(written, &line[run.shared.len()..]);
        }
        copied = at;
    }
    let mut named = named?;
    named.push_str(&text[copied..]);
    Some(finish(named, text))
}

fn neighbours(rest: &str) -> Neighbours<'_> {
    let mut run = Neighbours {
        lines: 0,
        shared: leading_folder(rest),
        length: 0,
    };
    if run.shared.len() < SHORTEST_WORTH_NAMING {
        return run;
    }
    for line in rest.split_inclusive(NEW_LINE) {
        let narrower = if line.starts_with(run.shared) {
            run.shared
        } else {
            folder_in_common(run.shared, leading_folder(line))
        };
        let gains_as_much = narrower.len() * (run.lines + 1) >= run.shared.len() * run.lines;
        if narrower.len() < SHORTEST_WORTH_NAMING || !gains_as_much {
            break;
        }
        run.shared = narrower;
        run.lines += 1;
        run.length += line.len();
    }
    run
}

fn line_start(before: &str) -> usize {
    before
        .rfind(NEW_LINE)
        .map_or(0, |end| end + NEW_LINE.len_utf8())
}

fn line_end(rest: &str) -> usize {
    rest.find(NEW_LINE).unwrap_or(rest.len())
}

fn starts_indented(rest: &str) -> bool {
    rest.lines()
        .next()
        .is_some_and(|next| next.starts_with(char::is_whitespace))
}

fn leading_folder(line: &str) -> &str {
    let end = line
        .bytes()
        .position(|byte| byte.is_ascii_whitespace() || byte == ENDS_A_PATH)
        .unwrap_or(line.len());
    folder_of(&line[..end])
}

fn folder_in_common<'a>(first: &'a str, second: &str) -> &'a str {
    let same = first
        .bytes()
        .zip(second.bytes())
        .take_while(|(one, other)| one == other)
        .count();
    let folder = first.as_bytes()[..same]
        .iter()
        .rposition(|&byte| byte == FOLDER_BYTE)
        .map_or(0, |last| last + 1);
    &first[..folder]
}

pub(crate) fn note(common: &str) -> String {
    format!("[paths under {common}]")
}

fn folder_of(path: &str) -> &str {
    path.rfind(FOLDER).map_or("", |at| &path[..=at])
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/shape/prefix.rs"]
mod tests;
