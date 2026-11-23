use crate::cut::Limits;
use crate::note::{NEW_LINE, left_out};

const CUT_CHARACTERS: &str = "chars";
const SMALLEST_CUT: usize = 40;
const BYTES_FOR_A_SPACE_OF_PROSE: usize = 12;
const SPACE: u8 = b' ';

pub(super) fn shorten(text: &str, limits: &Limits) -> Option<String> {
    if !text.split(NEW_LINE).any(|line| is_cut(line, limits)) {
        return None;
    }

    let mut shorter = String::with_capacity(text.len());
    for (at, line) in text.split(NEW_LINE).enumerate() {
        if at > 0 {
            shorter.push(NEW_LINE);
        }
        if is_cut(line, limits) {
            let (kept, cut) = line.split_at(line.floor_char_boundary(limits.longest_line));
            shorter.push_str(kept);
            shorter.push_str(&left_out(cut.chars().count(), CUT_CHARACTERS));
        } else {
            shorter.push_str(line);
        }
    }
    Some(shorter)
}

fn is_cut(line: &str, limits: &Limits) -> bool {
    line.len() >= limits.longest_line + SMALLEST_CUT && !(limits.spares_prose && is_prose(line))
}

fn is_prose(line: &str) -> bool {
    let spaces = line.bytes().filter(|&byte| byte == SPACE).count();
    spaces * BYTES_FOR_A_SPACE_OF_PROSE >= line.len()
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/cut/lines.rs"]
mod tests;
