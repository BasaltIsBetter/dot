use crate::cut::thin::thin;
use crate::note::{left_out, push_line};

const FEWEST_WORTH_THINNING: usize = 7;
const KEPT_AT_EACH_END: usize = 2;
const LINES_OF_THE_STACK: &str = "lines of the stack";
const CALLED_AT: &str = "at ";
const PYTHON_FRAME: &str = "File \"";

pub(crate) fn frames(text: &str) -> Option<String> {
    if !text.contains(CALLED_AT) && !text.contains(PYTHON_FRAME) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        (at..lines.len())
            .take_while(|&line| is_of_a_stack(&lines, line))
            .count()
    };
    thin(
        text,
        &lines,
        FEWEST_WORTH_THINNING,
        run_at,
        |thinner, stack| {
            let (innermost, rest) = stack.split_at(KEPT_AT_EACH_END);
            let (between, outermost) = rest.split_at(rest.len() - KEPT_AT_EACH_END);
            for line in innermost {
                push_line(thinner, line);
            }
            push_line(thinner, &left_out(between.len(), LINES_OF_THE_STACK));
            for line in outermost {
                push_line(thinner, line);
            }
        },
    )
}

fn is_of_a_stack(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let indented = line.starts_with(char::is_whitespace);
    let follows_a_python_frame = at
        .checked_sub(1)
        .is_some_and(|before| names_a_python_frame(lines[before]));
    indented
        && (line.trim_start().starts_with(CALLED_AT)
            || names_a_python_frame(line)
            || follows_a_python_frame)
}

fn names_a_python_frame(line: &str) -> bool {
    line.starts_with(char::is_whitespace) && line.trim_start().starts_with(PYTHON_FRAME)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/cut/thin/trace.rs"]
mod tests;
