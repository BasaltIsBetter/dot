use crate::note::{finish, push_line};
use crate::shape::prefix::{common_folder, note};

const FOLDER: char = '/';
const HERE: &str = "./";
const FEWEST_WORTH_GROUPING: usize = 6;
const PATHS_IN_TEN_LINES: usize = 9;
const NOT_IN_A_PATH: [char; 5] = [' ', '\t', ':', '[', '*'];
const NOT_IN_BRACES: [char; 3] = [',', '{', '}'];

pub(crate) fn group(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let paths: Vec<Option<&str>> = lines.iter().map(|line| path_of(line)).collect();
    let found = paths.iter().flatten().count();
    let written = lines.iter().filter(|line| !line.trim().is_empty()).count();
    if found < FEWEST_WORTH_GROUPING || found * 10 < written * PATHS_IN_TEN_LINES {
        return None;
    }

    let common = common_folder(paths.iter().flatten().copied()).unwrap_or_default();
    let mut grouped = String::with_capacity(text.len());
    if !common.is_empty() {
        push_line(&mut grouped, &note(common));
    }
    let mut at = 0;
    while at < lines.len() {
        let Some(path) = paths[at] else {
            push_line(&mut grouped, lines[at]);
            at += 1;
            continue;
        };
        let (folder, _) = split(path.strip_prefix(common).unwrap_or(path));
        let run = paths[at..]
            .iter()
            .map_while(|next| *next)
            .take_while(|next| split(next.strip_prefix(common).unwrap_or(next)).0 == folder)
            .count();
        push_folder(&mut grouped, common, &paths[at..at + run]);
        at += run;
    }

    let grouped = finish(grouped, text);
    (grouped.len() < text.len()).then_some(grouped)
}

fn path_of(line: &str) -> Option<&str> {
    let path = line.strip_prefix(HERE).unwrap_or(line);
    let is_path = path.contains(FOLDER) && !path.contains(NOT_IN_A_PATH) && !path.starts_with('-');
    is_path.then_some(path)
}

fn split(path: &str) -> (&str, &str) {
    path.rfind(FOLDER)
        .map_or(("", path), |at| path.split_at(at + 1))
}

fn push_folder(grouped: &mut String, common: &str, paths: &[Option<&str>]) {
    let shown: Vec<&str> = paths
        .iter()
        .flatten()
        .map(|path| path.strip_prefix(common).unwrap_or(path))
        .collect();
    let names: Vec<&str> = shown.iter().map(|path| split(path).1).collect();
    let foldable = names.len() > 1
        && names
            .iter()
            .all(|name| !name.is_empty() && !name.contains(NOT_IN_BRACES));
    if !foldable {
        for path in shown {
            push_line(grouped, path);
        }
        return;
    }
    let folder = shown.first().map_or("", |path| split(path).0);
    push_line(grouped, &format!("{folder}{{{}}}", names.join(",")));
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/shape/paths.rs"]
mod tests;
