use crate::cut::thin::thin;
use crate::note::{left_out, push_line};

const FEWEST_WORTH_THINNING: usize = 4;
const SIMILAR_LINES: &str = "similar lines";

pub(crate) fn lines(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        let first = lines[at];
        if first.trim().is_empty() {
            return 0;
        }
        let alike = lines[at + 1..]
            .iter()
            .take_while(|line| are_similar(first, line))
            .count();
        1 + alike
    };
    thin(
        text,
        &lines,
        FEWEST_WORTH_THINNING,
        run_at,
        |thinner, run| {
            if let [first, between @ .., last] = run {
                push_line(thinner, first);
                push_line(thinner, &left_out(between.len(), SIMILAR_LINES));
                push_line(thinner, last);
            }
        },
    )
}

fn are_similar(first: &str, second: &str) -> bool {
    let (first, second) = (first.as_bytes(), second.as_bytes());
    let (mut read, mut compared) = (0, 0);
    loop {
        match (first.get(read), second.get(compared)) {
            (None, None) => return true,
            (Some(byte), Some(other)) if byte.is_ascii_digit() && other.is_ascii_digit() => {
                read += number_length(&first[read..]);
                compared += number_length(&second[compared..]);
            }
            (Some(byte), Some(other)) if byte == other => {
                read += 1;
                compared += 1;
            }
            _ => return false,
        }
    }
}

fn number_length(line: &[u8]) -> usize {
    line.iter().take_while(|byte| byte.is_ascii_digit()).count()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/cut/thin/similar.rs"]
mod tests;
