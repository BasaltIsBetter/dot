use super::*;

#[test]
fn reads_letters_digits_and_punctuation() {
    assert!("feat/mac-app_2 é 漢".chars().all(is_readable));
}

#[test]
fn hides_control_characters() {
    for character in ['\n', '\u{1b}', '\u{0}', '\u{7f}'] {
        assert!(!is_readable(character), "{character:?}");
    }
}

#[test]
fn hides_invisible_characters_that_reorder_or_hide_text() {
    for character in [
        '\u{200b}', '\u{200f}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}', '\u{feff}',
    ] {
        assert!(!is_readable(character), "{character:?}");
    }
    assert!(is_readable('\u{2070}'));
    assert!(is_readable('\u{2065}'));
}
