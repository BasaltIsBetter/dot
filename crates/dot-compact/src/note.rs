pub(crate) const NEW_LINE: char = '\n';

pub(crate) fn left_out(count: usize, what: &str) -> String {
    format!("[+{count} {what}]")
}

pub(crate) fn push_line(text: &mut String, line: &str) {
    text.push_str(line);
    text.push(NEW_LINE);
}

pub(crate) fn finish(mut text: String, original: &str) -> String {
    if !original.ends_with(NEW_LINE) && text.ends_with(NEW_LINE) {
        text.pop();
    }
    text
}

#[cfg(test)]
#[path = "../../../tests/dot-compact/note.rs"]
mod tests;
