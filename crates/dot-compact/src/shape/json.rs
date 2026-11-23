use serde::de::IgnoredAny;

const OPENINGS: [char; 2] = ['{', '['];
const QUOTE: char = '"';
const ESCAPE: char = '\\';
const INDENTED: &str = "\n ";
const INDENTED_WITH_TABS: &str = "\n\t";
const NEW_LINE: char = '\n';
const FIRST_LINES: [&str; 2] = ["{", "["];
const CLOSINGS: [char; 2] = ['}', ']'];
const MOST_TRIES: usize = 16;

pub(crate) fn is_document(text: &str) -> bool {
    let document = text.trim();
    document.starts_with(OPENINGS) && serde_json::from_str::<IgnoredAny>(document).is_ok()
}

pub(crate) fn minify(text: &str) -> Option<String> {
    let document = text.trim();
    let indented = document.contains(INDENTED) || document.contains(INDENTED_WITH_TABS);
    if !indented || !is_document(document) {
        return None;
    }

    let mut minified = String::with_capacity(document.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in document.chars() {
        if in_string {
            minified.push(character);
            in_string = escaped || character != QUOTE;
            escaped = !escaped && character == ESCAPE;
            continue;
        }
        if character.is_ascii_whitespace() {
            continue;
        }
        in_string = character == QUOTE;
        minified.push(character);
    }
    Some(minified)
}

pub(crate) fn minify_after_lines(text: &str) -> Option<String> {
    if !text.trim_end().ends_with(CLOSINGS) {
        return None;
    }
    let mut start = 0;
    let mut tries = 0;
    for line in text.split_inclusive(NEW_LINE) {
        let opens_a_document = start > 0 && FIRST_LINES.contains(&line.trim_end());
        if opens_a_document {
            tries += 1;
            if tries > MOST_TRIES {
                return None;
            }
            if let Some(minified) = minify(&text[start..]) {
                return Some(format!("{}{minified}", &text[..start]));
            }
        }
        start += line.len();
    }
    None
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/shape/json.rs"]
mod tests;
