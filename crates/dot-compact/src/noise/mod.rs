mod repeat;
mod ruler;

use std::borrow::Cow;
use std::iter::Peekable;
use std::str::Chars;

use crate::note::NEW_LINE;

pub(crate) use self::repeat::collapse;
pub(crate) use self::ruler::shorten as shorten_rulers;

const ESCAPE: char = '\u{1b}';
const BELL: char = '\u{7}';
const CARRIAGE_RETURN: char = '\r';
const NOISY_BYTES: [u8; 2] = [0x1b, b'\r'];
const CONTROL_SEQUENCE: char = '[';
const SYSTEM_COMMAND: char = ']';
const STRING_END: char = '\\';
const FIRST_FINAL: char = '@';
const LAST_FINAL: char = '~';
const INTERMEDIATES: std::ops::RangeInclusive<char> = ' '..='/';

pub(crate) fn clean(output: &str) -> Cow<'_, str> {
    let bytes = output.as_bytes();
    let noisy = NOISY_BYTES.iter().any(|noisy| bytes.contains(noisy));
    if !noisy {
        return Cow::Borrowed(output);
    }

    let mut cleaned = String::with_capacity(output.len());
    for (index, line) in output.split(NEW_LINE).enumerate() {
        if index > 0 {
            cleaned.push(NEW_LINE);
        }
        strip_escapes(last_overwrite(line), &mut cleaned);
    }
    Cow::Owned(cleaned)
}

pub(crate) fn drawn_over(unfinished: &str) -> usize {
    unfinished
        .trim_end_matches(CARRIAGE_RETURN)
        .rfind(CARRIAGE_RETURN)
        .map_or(0, |last| last + CARRIAGE_RETURN.len_utf8())
}

fn last_overwrite(line: &str) -> &str {
    line.rsplit(CARRIAGE_RETURN)
        .find(|drawn| !drawn.is_empty())
        .unwrap_or_default()
}

fn strip_escapes(line: &str, into: &mut String) {
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character != ESCAPE {
            into.push(character);
            continue;
        }
        match characters.next() {
            Some(CONTROL_SEQUENCE) => skip_control_sequence(&mut characters),
            Some(SYSTEM_COMMAND) => skip_system_command(&mut characters),
            Some(first) if INTERMEDIATES.contains(&first) => skip_designation(&mut characters),
            _ => {}
        }
    }
}

fn skip_control_sequence(characters: &mut Peekable<Chars<'_>>) {
    for character in characters {
        if (FIRST_FINAL..=LAST_FINAL).contains(&character) {
            return;
        }
    }
}

fn skip_designation(characters: &mut Peekable<Chars<'_>>) {
    for character in characters {
        if !INTERMEDIATES.contains(&character) {
            return;
        }
    }
}

fn skip_system_command(characters: &mut Peekable<Chars<'_>>) {
    while let Some(character) = characters.next() {
        let ends_here = character == BELL
            || (character == ESCAPE && characters.next_if_eq(&STRING_END).is_some());
        if ends_here {
            return;
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/noise/mod.rs"]
mod tests;
