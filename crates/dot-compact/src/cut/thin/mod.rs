mod found;
mod git;
mod similar;
mod trace;

use crate::note::finish;

pub(super) type Thinning = fn(&str) -> Option<String>;

pub(super) const OF_AN_OUTPUT: &[Thinning] = &[
    trace::frames,
    git::commits,
    git::messages,
    found::names_of_a_folder,
    found::entries_of_a_listing,
    similar::lines,
    found::matches_of_a_file,
];

pub(super) const OF_A_DIFF: &[Thinning] = &[git::far_context];

fn thin<'a>(
    text: &'a str,
    lines: &[&'a str],
    fewest: usize,
    run_at: impl Fn(usize) -> usize,
    write: impl Fn(&mut String, &[&'a str]),
) -> Option<String> {
    let start_of = |at: usize| {
        lines.get(at).map_or(text.len(), |line| {
            line.as_ptr().addr() - text.as_ptr().addr()
        })
    };
    let mut thinner: Option<String> = None;
    let mut copied = 0;
    let mut at = 0;

    while at < lines.len() {
        let run = run_at(at);
        if run >= fewest {
            let thinner = thinner.get_or_insert_with(|| String::with_capacity(text.len()));
            thinner.push_str(&text[start_of(copied)..start_of(at)]);
            write(thinner, &lines[at..at + run]);
            copied = at + run;
        }
        at += run.max(1);
    }

    let mut thinner = thinner?;
    thinner.push_str(&text[start_of(copied)..]);
    Some(finish(thinner, text))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/cut/thin/mod.rs"]
mod tests;
