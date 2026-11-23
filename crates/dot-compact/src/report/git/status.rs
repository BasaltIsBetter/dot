use crate::command::CommandLine;
use crate::note::{finish, push_line};
use crate::report::git::GIT;

const STATUS: &str = "status";
const STARTS: [&str; 2] = ["On branch ", "HEAD detached "];
const HINT: &str = "  (";
const ENTRY: char = '\t';
const NOTHING_ADDED: &str = "no changes added to commit";
const SENTENCES: [&str; 16] = [
    "All conflicts fixed",
    "Ignored files:",
    "Initial commit",
    "It took ",
    "Last command",
    "Next command",
    "No commands remaining",
    "No commits yet",
    "Not currently on any branch",
    "You are ",
    "You have ",
    "Your branch ",
    "and have ",
    "interactive rebase in progress",
    "nothing added to commit",
    "nothing to commit",
];
const SECTIONS: [(&str, &str); 4] = [
    ("Changes to be committed:", "staged:"),
    ("Changes not staged for commit:", "unstaged:"),
    ("Untracked files:", "untracked:"),
    ("Unmerged paths:", "unmerged:"),
];
const CHANGES: [(&str, &str); 8] = [
    ("modified:", "M"),
    ("new file:", "A"),
    ("deleted:", "D"),
    ("renamed:", "R"),
    ("copied:", "C"),
    ("typechange:", "T"),
    ("both modified:", "UU"),
    ("both added:", "AA"),
];

pub(crate) fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    let is_status =
        command.runs_with(GIT, STATUS) && STARTS.iter().any(|start| text.starts_with(start));
    if !is_status {
        return None;
    }

    let mut shorter = String::with_capacity(text.len());
    let mut inside = true;
    for line in text.lines() {
        inside = inside && is_of_the_status(line);
        if !inside {
            push_line(&mut shorter, line);
            continue;
        }
        let dropped =
            line.trim().is_empty() || line.starts_with(HINT) || line.starts_with(NOTHING_ADDED);
        if dropped {
            continue;
        }
        let section = SECTIONS.iter().find(|(long, _)| line == *long);
        match (section, line.strip_prefix(ENTRY)) {
            (Some((_, short)), _) => push_line(&mut shorter, short),
            (None, Some(entry)) => push_line(&mut shorter, &change(entry)),
            (None, None) => push_line(&mut shorter, line),
        }
    }
    Some(finish(shorter, text))
}

fn is_of_the_status(line: &str) -> bool {
    let starts_with_any = |starts: &[&str]| starts.iter().any(|start| line.starts_with(start));
    line.trim().is_empty()
        || line.starts_with(ENTRY)
        || SECTIONS.iter().any(|(long, _)| line == *long)
        || starts_with_any(&[HINT, NOTHING_ADDED])
        || starts_with_any(&STARTS)
        || starts_with_any(&SENTENCES)
}

fn change(entry: &str) -> String {
    CHANGES
        .iter()
        .find_map(|(long, short)| {
            let path = entry.strip_prefix(long)?;
            Some(format!("{short} {}", path.trim_start()))
        })
        .unwrap_or_else(|| entry.to_owned())
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/git/status.rs"]
mod tests;
