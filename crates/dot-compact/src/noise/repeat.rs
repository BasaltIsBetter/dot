use crate::note::{finish, push_line};

const REPEATS_WORTH_COUNTING: usize = 3;

pub(crate) fn collapse(text: &str) -> Option<String> {
    if !repeats_a_line(text) {
        return None;
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut collapsed = String::with_capacity(text.len());
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        let run = lines[at..]
            .iter()
            .take_while(|&&other| other == line)
            .count();
        at += run;

        if run >= REPEATS_WORTH_COUNTING && !is_blank(line) {
            push_line(&mut collapsed, &format!("{line} [×{run}]"));
        } else {
            (0..run).for_each(|_| push_line(&mut collapsed, line));
        }
    }
    Some(finish(collapsed, text))
}

fn repeats_a_line(text: &str) -> bool {
    let mut previous = None;
    let mut run = 0;
    text.lines().any(|line| {
        run = if previous == Some(line) { run + 1 } else { 1 };
        previous = Some(line);
        run >= REPEATS_WORTH_COUNTING && !is_blank(line)
    })
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/noise/repeat.rs"]
mod tests;
