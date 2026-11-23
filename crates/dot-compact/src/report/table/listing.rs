use crate::command::CommandLine;
use crate::note::{finish, push_line};

const LS: &str = "ls";
const FEWEST_WORTH_REWRITING: usize = 4;
const KINDS: &str = "-bcdlps";
const PERMISSIONS: usize = 9;
const FIELDS_BEFORE_NAME: usize = 8;
const REGULAR: char = '-';
const OWNERS_VARY: &str = "[ls -l as: mode owner:group size date name]";

struct Entry<'a> {
    kind: char,
    mode: u16,
    mark: &'a str,
    owners: (&'a str, &'a str),
    size: &'a str,
    date: [&'a str; 3],
    name: &'a str,
}

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    if !command.mentions(LS) {
        return None;
    }
    let entries: Vec<Option<Entry<'_>>> = text.lines().map(entry).collect();
    let first = entries.iter().flatten().next()?;
    if entries.iter().flatten().count() < FEWEST_WORTH_REWRITING {
        return None;
    }

    let same_owners = entries
        .iter()
        .flatten()
        .all(|entry| entry.owners == first.owners);
    let header = if same_owners {
        format!(
            "[ls -l as: mode size date name; all owned by {}:{}]",
            first.owners.0, first.owners.1
        )
    } else {
        OWNERS_VARY.to_owned()
    };
    let mut shorter = String::with_capacity(text.len());
    let mut announced = false;
    for (line, entry) in text.lines().zip(&entries) {
        let Some(entry) = entry else {
            push_line(&mut shorter, line);
            continue;
        };
        if !announced {
            push_line(&mut shorter, &header);
            announced = true;
        }
        push_line(&mut shorter, &written(entry, same_owners));
    }
    Some(finish(shorter, text))
}

fn entry(line: &str) -> Option<Entry<'_>> {
    let mut rest = line;
    let mut fields = [""; FIELDS_BEFORE_NAME];
    for field in &mut fields {
        rest = rest.trim_start();
        let end = rest.find(char::is_whitespace)?;
        (*field, rest) = rest.split_at(end);
    }
    let name = rest.strip_prefix(' ')?;
    let [flags, links, owner, group, size, month, day, time] = fields;

    let kind = flags.chars().next().filter(|kind| KINDS.contains(*kind))?;
    let mode = mode_of(flags.get(1..=PERMISSIONS)?)?;
    let counted = links.bytes().all(|byte| byte.is_ascii_digit()) && is_size(size);
    (counted && !name.is_empty()).then_some(Entry {
        kind,
        mode,
        mark: flags.get(PERMISSIONS + 1..).unwrap_or_default(),
        owners: (owner, group),
        size,
        date: [month, day, time],
        name,
    })
}

fn is_size(size: &str) -> bool {
    let digits = size.trim_end_matches(|unit: char| unit.is_ascii_alphabetic());
    let has_one_unit_at_most = size.len() - digits.len() <= 1;
    let is_a_number = digits.starts_with(|first: char| first.is_ascii_digit())
        && digits
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.');
    has_one_unit_at_most && is_a_number
}

fn mode_of(permissions: &str) -> Option<u16> {
    let mut mode = 0;
    for (place, flag) in permissions.chars().enumerate() {
        let bit = 1 << (PERMISSIONS - 1 - place);
        let special = 1 << (PERMISSIONS + 2 - place / 3);
        mode |= match flag {
            'r' | 'w' | 'x' => bit,
            's' | 't' => bit + special,
            'S' | 'T' => special,
            '-' => 0,
            _ => return None,
        };
    }
    Some(mode)
}

fn written(entry: &Entry<'_>, same_owners: bool) -> String {
    let Entry {
        kind,
        mode,
        mark,
        owners,
        size,
        date,
        name,
    } = entry;
    let kind = if *kind == REGULAR {
        String::new()
    } else {
        kind.to_string()
    };
    let owners = if same_owners {
        String::new()
    } else {
        format!(" {}:{}", owners.0, owners.1)
    };
    format!(
        "{kind}{mode:o}{mark}{owners} {size} {} {} {} {name}",
        date[0], date[1], date[2]
    )
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/table/listing.rs"]
mod tests;
