const PREFIXES: [&str; 22] = [
    "!", "builtin", "command", "do", "elif", "else", "env", "exec", "if", "nice", "noglob",
    "nohup", "proxy", "rtk", "sudo", "then", "time", "until", "while", "xargs", "{", "}",
];
const NESTED: [&str; 3] = ["-exec", "-execdir", "-ok"];
const READS_ITS_ARGUMENTS: &str = "xargs";
const QUOTES: [char; 2] = ['\'', '"'];
const ESCAPE: char = '\\';
const PIPE: char = '|';
const BACKGROUND: char = '&';
const REDIRECTION: char = '>';
const OPTION: char = '-';
const ASSIGNMENT: char = '=';
const FOLDER: char = '/';

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation<'a> {
    pub(crate) program: &'a str,
    pub(crate) arguments: Vec<&'a str>,
    pub(crate) piped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    Pipe,
    Break,
}

pub(crate) fn invocations(line: &str) -> Vec<Invocation<'_>> {
    let mut found = Vec::new();
    let mut current: Option<Invocation<'_>> = None;
    let mut piped = false;

    for token in tokens(line) {
        match token {
            Token::Word(word) => {
                let nested = current.is_some() && NESTED.contains(&word);
                if nested {
                    found.extend(current.take());
                    piped = false;
                } else if let Some(invocation) = current.as_mut() {
                    invocation.arguments.push(word);
                } else {
                    piped &= word != READS_ITS_ARGUMENTS;
                    current = program_of(word).map(|program| started(program, piped));
                }
            }
            Token::Pipe | Token::Break => {
                found.extend(current.take());
                piped = token == Token::Pipe;
            }
        }
    }
    found.extend(current);
    found
}

const fn started(program: &str, piped: bool) -> Invocation<'_> {
    Invocation {
        program,
        arguments: Vec::new(),
        piped,
    }
}

fn program_of(word: &str) -> Option<&str> {
    let bare = word.trim_matches(QUOTES);
    let skipped = PREFIXES.contains(&bare)
        || bare.starts_with(OPTION)
        || bare.bytes().all(|byte| byte.is_ascii_digit())
        || is_assignment(bare);
    if skipped {
        return None;
    }
    bare.rsplit(FOLDER).next().filter(|name| !name.is_empty())
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once(ASSIGNMENT) else {
        return false;
    };
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn tokens(line: &str) -> Vec<Token<'_>> {
    let mut found = Vec::new();
    let mut start: Option<usize> = None;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut previous = ' ';
    let mut characters = line.char_indices().peekable();

    while let Some((at, character)) = characters.next() {
        let next = characters.peek().map(|&(_, next)| next);
        let in_word = escaped || quote.is_some() || !ends_word(character, previous, next);
        if in_word {
            start.get_or_insert(at);
            (quote, escaped) = quoting(character, quote, escaped);
        } else {
            found.extend(start.take().map(|from| Token::Word(&line[from..at])));
            push_operator(&mut found, character);
        }
        previous = character;
    }
    found.extend(start.map(|from| Token::Word(&line[from..])));
    found
}

fn ends_word(character: char, previous: char, next: Option<char>) -> bool {
    match character {
        BACKGROUND => previous != REDIRECTION && next != Some(REDIRECTION),
        PIPE | ';' | '(' | ')' | '`' => true,
        other => other.is_whitespace(),
    }
}

fn quoting(character: char, quote: Option<char>, escaped: bool) -> (Option<char>, bool) {
    if escaped {
        return (quote, false);
    }
    match (character, quote) {
        (ESCAPE, None | Some('"')) => (quote, true),
        (closing, Some(open)) if closing == open => (None, false),
        (opening, None) if QUOTES.contains(&opening) => (Some(opening), false),
        _ => (quote, false),
    }
}

fn push_operator(found: &mut Vec<Token<'_>>, character: char) {
    if character == PIPE {
        let doubled = found.last() == Some(&Token::Pipe);
        if doubled {
            found.pop();
        }
        found.push(if doubled { Token::Break } else { Token::Pipe });
        return;
    }
    let pipes_the_errors_too = character == BACKGROUND && found.last() == Some(&Token::Pipe);
    let ends_a_command = !character.is_whitespace() || character == '\n';
    if ends_a_command && !pipes_the_errors_too {
        found.push(Token::Break);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/command/split.rs"]
mod tests;
