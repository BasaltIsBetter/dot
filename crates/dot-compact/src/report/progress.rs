use crate::command::CommandLine;
use crate::note::{finish, left_out, push_line};
use crate::report::rules::{RULES, Rule};

const SHORTEST_RUN_WORTH_COUNTING: usize = 3;

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    let rules = active(command);
    if rules.is_empty() {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let (shorter, changed) = shortened(&rules, &lines);
    changed.then(|| finish(shorter, text))
}

pub(crate) fn active(command: &CommandLine<'_>) -> Vec<&'static Rule> {
    RULES
        .iter()
        .flat_map(|group| group.iter())
        .filter(|rule| rule.tools.iter().any(|tool| command.mentions(tool)))
        .collect()
}

pub(crate) fn shortened(rules: &[&Rule], lines: &[&str]) -> (String, bool) {
    let dropped = |at: usize| {
        rules
            .iter()
            .find(|rule| rule.drops(lines, at))
            .map(|rule| rule.what)
    };
    let mut shorter = String::new();
    let mut changed = false;
    let mut at = 0;
    while at < lines.len() {
        let Some(what) = dropped(at) else {
            push_line(&mut shorter, lines[at]);
            at += 1;
            continue;
        };
        let run = (at..lines.len())
            .take_while(|&next| dropped(next) == Some(what))
            .count();
        if run < SHORTEST_RUN_WORTH_COUNTING {
            for line in &lines[at..at + run] {
                push_line(&mut shorter, line);
            }
        } else {
            push_line(&mut shorter, &left_out(run, what));
            changed = true;
        }
        at += run;
    }
    (shorter, changed)
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/report/progress.rs"]
mod tests;
