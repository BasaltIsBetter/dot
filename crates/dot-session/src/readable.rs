use std::ops::RangeInclusive;

const INVISIBLE_FORMATS: [RangeInclusive<char>; 4] = [
    '\u{200b}'..='\u{200f}',
    '\u{202a}'..='\u{202e}',
    '\u{2066}'..='\u{2069}',
    '\u{feff}'..='\u{feff}',
];

pub(crate) fn is_readable(character: char) -> bool {
    !character.is_control()
        && !INVISIBLE_FORMATS
            .iter()
            .any(|formats| formats.contains(&character))
}

#[cfg(test)]
#[path = "../../../tests/dot-session/readable.rs"]
mod tests;
