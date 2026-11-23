use crate::cut::thin::thin;
use crate::note::{finish, left_out, push_line};

const KEPT_OF_A_FILE: usize = 5;
const KEPT_OF_A_FOLDER: usize = 4;
const FEWEST_LEFT_OUT: usize = 3;
const FEWEST_LINES_OF_A_FILE: usize = 1 + KEPT_OF_A_FILE + FEWEST_LEFT_OUT;
const LINES_OF_THIS_FILE: &str = "lines of this file";
const NAMES_START: char = '{';
const NAMES_END: char = '}';
const NAME_BREAK: char = ',';
const ROW_BREAKS: [char; 2] = [':', '-'];
const LISTING: &str = "[ls -l as: mode size date name;";
const NAMES_AND_SIZES: &str = "[ls -l as: name size;";
const FIELDS_OF_AN_ENTRY: usize = 6;
const FOLDER: char = 'd';
const ITSELF_AND_ITS_PARENT: [&str; 2] = [".", ".."];

pub(crate) fn matches_of_a_file(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        if !heads_a_file(lines[at]) {
            return 0;
        }
        let rows = lines[at + 1..]
            .iter()
            .take_while(|line| is_row(line))
            .count();
        1 + rows
    };
    thin(
        text,
        &lines,
        FEWEST_LINES_OF_A_FILE,
        run_at,
        |thinner, file| {
            let (kept, cut) = file.split_at(1 + KEPT_OF_A_FILE);
            for line in kept {
                push_line(thinner, line);
            }
            push_line(thinner, &left_out(cut.len(), LINES_OF_THIS_FILE));
        },
    )
}

pub(crate) fn names_of_a_folder(text: &str) -> Option<String> {
    if !text.contains(NAMES_START) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        lines[at..]
            .iter()
            .take_while(|line| names_to_spare(line).is_some())
            .count()
    };
    thin(text, &lines, 1, run_at, |thinner, folders| {
        for (folder, names) in folders.iter().filter_map(|line| names_to_spare(line)) {
            push_fewer_names(thinner, folder, names);
        }
    })
}

pub(crate) fn entries_of_a_listing(text: &str) -> Option<String> {
    if !text.contains(LISTING) {
        return None;
    }
    let mut thinner = String::with_capacity(text.len());
    let mut is_listing = false;
    for line in text.lines() {
        if let Some(owners) = line.strip_prefix(LISTING) {
            push_line(&mut thinner, &format!("{NAMES_AND_SIZES}{owners}"));
            is_listing = true;
            continue;
        }
        match entry(line).filter(|_| is_listing) {
            Some((_, _, name)) if ITSELF_AND_ITS_PARENT.contains(&name) => {}
            Some((true, _, name)) => push_line(&mut thinner, &format!("{name}/")),
            Some((false, size, name)) => push_line(&mut thinner, &format!("{name} {size}")),
            None => push_line(&mut thinner, line),
        }
    }
    Some(finish(thinner, text))
}

fn entry(line: &str) -> Option<(bool, &str, &str)> {
    let fields: Vec<&str> = line.splitn(FIELDS_OF_AN_ENTRY, ' ').collect();
    let [mode, size, month, _, _, name] = fields.as_slice() else {
        return None;
    };
    let has_a_mode = mode
        .trim_start_matches(|kind: char| kind.is_ascii_lowercase())
        .starts_with(|digit: char| digit.is_ascii_digit());
    let has_a_size = size.starts_with(|digit: char| digit.is_ascii_digit());
    let has_a_month = month.bytes().all(|byte| byte.is_ascii_alphabetic()) && !month.is_empty();
    (has_a_mode && has_a_size && has_a_month && !name.is_empty()).then_some((
        mode.starts_with(FOLDER),
        size,
        name,
    ))
}

fn is_row(line: &str) -> bool {
    line.split_once(ROW_BREAKS).is_some_and(|(number, _)| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn heads_a_file(line: &str) -> bool {
    !line.is_empty() && !line.contains(char::is_whitespace) && !is_row(line)
}

fn names_to_spare(line: &str) -> Option<(&str, &str)> {
    let (folder, names) = line.strip_suffix(NAMES_END)?.split_once(NAMES_START)?;
    let written = names.split(NAME_BREAK).count();
    let spares = written >= KEPT_OF_A_FOLDER + FEWEST_LEFT_OUT;
    (spares && !line.contains(char::is_whitespace)).then_some((folder, names))
}

fn push_fewer_names(thinner: &mut String, folder: &str, names: &str) {
    let written = names.split(NAME_BREAK).count();
    thinner.push_str(folder);
    thinner.push(NAMES_START);
    for name in names.split(NAME_BREAK).take(KEPT_OF_A_FOLDER) {
        thinner.push_str(name);
        thinner.push(NAME_BREAK);
    }
    push_line(
        thinner,
        &format!("+{} more{NAMES_END}", written - KEPT_OF_A_FOLDER),
    );
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/cut/thin/found.rs"]
mod tests;
