use crate::note::{finish, push_line};

const BRANCHES: [&str; 2] = ["├── ", "└── "];
const END_OF_A_BRANCH: &str = "── ";
const TRUNK: char = '│';
const DRAWN: [char; 3] = ['│', '├', '└'];
const GAPS: [char; 2] = [' ', '\u{a0}'];
const GAPS_AFTER_A_TRUNK: usize = 3;
const FEWEST_WORTH_FLATTENING: usize = 4;
const NOTE: &str = "[tree as: one space per level]";
const LEVEL: &str = " ";

#[derive(Clone, Copy)]
struct Limb<'a> {
    depth: usize,
    name: &'a str,
}

pub(crate) fn flatten(text: &str) -> Option<String> {
    if !text.contains(END_OF_A_BRANCH) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let branches: Vec<Option<Limb<'_>>> = lines.iter().map(|line| branch(line)).collect();
    let first = branches.iter().position(Option::is_some)?;
    let last = branches.iter().rposition(Option::is_some)?;
    if branches.iter().flatten().count() < FEWEST_WORTH_FLATTENING {
        return None;
    }

    let mut flat = String::with_capacity(text.len());
    push_line(&mut flat, NOTE);
    for (at, line) in lines.iter().enumerate() {
        let in_the_tree = (first..=last).contains(&at);
        let limb = match branches[at] {
            Some(branch) => branch,
            None if in_the_tree => hanging(line),
            None => Limb {
                depth: 0,
                name: line,
            },
        };
        if in_the_tree && limb.name.contains(DRAWN) {
            return None;
        }

        flat.push_str(&LEVEL.repeat(limb.depth));
        push_line(&mut flat, limb.name);
    }
    Some(finish(flat, text))
}

fn branch(line: &str) -> Option<Limb<'_>> {
    let (depth, rest) = below_the_trunks(line);
    let name = BRANCHES
        .iter()
        .find_map(|branch| rest.strip_prefix(branch))?;
    Some(Limb {
        depth: depth + 1,
        name,
    })
}

fn hanging(line: &str) -> Limb<'_> {
    let (depth, name) = below_the_trunks(line);
    Limb { depth, name }
}

fn below_the_trunks(line: &str) -> (usize, &str) {
    let mut depth = 0;
    let mut rest = line;
    while let Some(below) = below_a_trunk(rest) {
        depth += 1;
        rest = below;
    }
    (depth, rest)
}

fn below_a_trunk(line: &str) -> Option<&str> {
    let mut characters = line.chars();
    let first = characters.next()?;
    let is_a_level = (first == TRUNK || GAPS.contains(&first))
        && (0..GAPS_AFTER_A_TRUNK)
            .all(|_| characters.next().is_some_and(|gap| GAPS.contains(&gap)));
    is_a_level.then_some(characters.as_str())
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/shape/tree.rs"]
mod tests;
