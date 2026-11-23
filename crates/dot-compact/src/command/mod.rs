mod split;

use dot_api::{RAW_OUTPUT_MARK, WHOLE_OUTPUTS_FOLDER};

use self::split::{Invocation, invocations};

const READERS: [&str; 42] = [
    "awk", "base64", "bat", "bzcat", "cat", "cmp", "column", "comm", "cut", "diff", "expand",
    "fmt", "fold", "gawk", "gzcat", "head", "hexdump", "join", "jq", "less", "more", "nl", "od",
    "paste", "patch", "pbpaste", "perl", "plutil", "read", "rev", "sed", "sops", "sort", "strings",
    "tac", "tail", "tr", "uniq", "unzip", "xxd", "xzcat", "zcat",
];
const EDITS_IN_PLACE: &str = "-i";
const CARRIERS: [&str; 14] = [
    "bash", "dash", "docker", "gtimeout", "kubectl", "nerdctl", "oc", "podman", "sh", "ssh", "su",
    "timeout", "watch", "zsh",
];
const QUOTES: [char; 2] = ['\'', '"'];
const GIT: &str = "git";
const GIT_CONTENT: [&str; 9] = [
    "annotate",
    "apply",
    "blame",
    "cat-file",
    "diff",
    "format-patch",
    "range-diff",
    "show",
    "stash",
];
const GIT_PATCH: [&str; 2] = ["-p", "--patch"];
const GIT_NAMES_ONLY: &str = "--name-only";
const FOLDER: char = '/';

pub(crate) struct CommandLine<'a> {
    invocations: Vec<Invocation<'a>>,
    raw: bool,
}

impl<'a> CommandLine<'a> {
    pub(crate) fn read(line: &'a str) -> Self {
        Self {
            invocations: invocations(line),
            raw: line.split_whitespace().any(|word| word == RAW_OUTPUT_MARK)
                || line.contains(WHOLE_OUTPUTS_FOLDER),
        }
    }

    pub(crate) const fn keeps_raw(&self) -> bool {
        self.raw
    }

    pub(crate) fn prints_content(&self) -> bool {
        self.invocations
            .iter()
            .filter(|invocation| !invocation.piped)
            .any(|invocation| {
                reads_a_file(invocation)
                    || shows_git_content(invocation)
                    || carries_a_reader(invocation)
            })
    }

    pub(crate) fn mentions(&self, word: &str) -> bool {
        self.invocations.iter().any(|invocation| {
            invocation.program == word
                || invocation
                    .arguments
                    .iter()
                    .any(|argument| argument.rsplit(FOLDER).next() == Some(word))
        })
    }

    pub(crate) fn runs_with(&self, program: &str, argument: &str) -> bool {
        self.invocations.iter().any(|invocation| {
            invocation.program == program && invocation.arguments.contains(&argument)
        })
    }
}

fn reads_a_file(invocation: &Invocation<'_>) -> bool {
    let edits_in_place = invocation
        .arguments
        .iter()
        .any(|argument| argument.starts_with(EDITS_IN_PLACE));
    READERS.contains(&invocation.program) && !edits_in_place
}

fn carries_a_reader(invocation: &Invocation<'_>) -> bool {
    if !CARRIERS.contains(&invocation.program) {
        return false;
    }
    let arguments = &invocation.arguments;
    let named = arguments
        .iter()
        .position(|argument| READERS.contains(argument))
        .and_then(|at| arguments[at..].split_first())
        .is_some_and(|(program, arguments)| {
            reads_a_file(&Invocation {
                program,
                arguments: arguments.to_vec(),
                piped: false,
            })
        });
    named || arguments.iter().any(|argument| quotes_a_reader(argument))
}

fn quotes_a_reader(argument: &str) -> bool {
    QUOTES.iter().any(|&quote| {
        argument
            .strip_prefix(quote)
            .and_then(|open| open.strip_suffix(quote))
            .is_some_and(|script| CommandLine::read(script).prints_content())
    })
}

fn shows_git_content(invocation: &Invocation<'_>) -> bool {
    let asks = |wanted: &[&str]| {
        invocation
            .arguments
            .iter()
            .any(|argument| wanted.contains(argument))
    };
    let lists_names_only = asks(&[GIT_NAMES_ONLY]) && !asks(&GIT_PATCH);
    invocation.program == GIT && (asks(&GIT_CONTENT) || asks(&GIT_PATCH)) && !lists_names_only
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/command/mod.rs"]
mod tests;
