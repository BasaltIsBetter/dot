use crate::cut::{Limits, outline};
use crate::note::{NEW_LINE, push_line};

const LINE_BREAK: u8 = b'\n';
const SHARE_OF_THE_CUT_FOR_ITS_OUTLINE: usize = 4;

pub(super) fn cut(text: &str, limits: &Limits) -> Option<String> {
    if text.len() <= limits.cut_above {
        return None;
    }
    let bytes = text.as_bytes();
    let head = after_the_last_line_break(&bytes[..limits.kept_from_the_start])?;
    let kept_from_the_end = limits.kept - limits.kept_from_the_start;
    let tail = before_the_tail(bytes, text.len() - kept_from_the_end);
    let middle = text.get(head..tail).filter(|middle| !middle.is_empty())?;

    let first = text[..head].matches(NEW_LINE).count() + 1;
    let last = first + middle.lines().count() - 1;
    let opened = if limits.outlined {
        outline::of(
            middle,
            first,
            middle.len() / SHARE_OF_THE_CUT_FOR_ITS_OUTLINE,
        )
    } else {
        Vec::new()
    };

    let mut cut = String::with_capacity(limits.kept + middle.len() / 2);
    cut.push_str(&text[..head]);
    push_note(&mut cut, first, last, &opened);
    cut.push_str(&text[tail..]);
    Some(cut)
}

fn after_the_last_line_break(start: &[u8]) -> Option<usize> {
    start
        .iter()
        .rposition(|&byte| byte == LINE_BREAK)
        .map(|at| at + 1)
}

fn before_the_tail(bytes: &[u8], wanted: usize) -> usize {
    bytes[wanted - 1..]
        .iter()
        .position(|&byte| byte == LINE_BREAK)
        .map_or(bytes.len(), |at| wanted + at)
}

fn push_note(cut: &mut String, first: usize, last: usize, opened: &[String]) {
    let lines = last + 1 - first;
    if opened.is_empty() {
        push_line(
            cut,
            &format!("[+{lines} lines cut: {first} to {last} of this output]"),
        );
        return;
    }
    push_line(
        cut,
        &format!(
            "[+{lines} lines cut: {first} to {last} of this output; among them, these open a block]"
        ),
    );
    for line in opened {
        push_line(cut, line);
    }
    push_line(cut, &format!("[the output goes on at line {}]", last + 1));
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/cut/middle.rs"]
mod tests;
