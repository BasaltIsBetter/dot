const BEFORE_COMMAND: &str = " && eval ";
const AFTER_COMMAND: &str = " < /dev/null && pwd -P >| ";
const BASH_SNAPSHOT: &str = "/shell-snapshots/snapshot-bash-";
const ZSH_SNAPSHOT: &str = "/shell-snapshots/snapshot-zsh-";
const SINGLE_QUOTE: char = '\'';
const DOUBLE_QUOTE: char = '"';
const ESCAPE: char = '\\';
const ESCAPED_IN_DOUBLE_QUOTES: [char; 4] = ['"', '\\', '$', '`'];
const ENDS_A_WORD: [char; 7] = [';', '&', '|', '<', '>', '(', ')'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Snapshot {
    Bash,
    Zsh,
}

pub(crate) fn asked(script: &str) -> Option<String> {
    let (before, _) = script.rsplit_once(AFTER_COMMAND)?;
    let (_, quoted) = before.split_once(BEFORE_COMMAND)?;
    unquoted(quoted)
}

pub(crate) fn snapshot(script: &str) -> Option<Snapshot> {
    if script.contains(BASH_SNAPSHOT) {
        return Some(Snapshot::Bash);
    }
    script.contains(ZSH_SNAPSHOT).then_some(Snapshot::Zsh)
}

fn unquoted(word: &str) -> Option<String> {
    let mut plain = String::with_capacity(word.len());
    let mut characters = word.chars();
    while let Some(character) = characters.next() {
        match character {
            SINGLE_QUOTE => loop {
                match characters.next()? {
                    SINGLE_QUOTE => break,
                    quoted => plain.push(quoted),
                }
            },
            DOUBLE_QUOTE => loop {
                match characters.next()? {
                    DOUBLE_QUOTE => break,
                    ESCAPE => {
                        let escaped = characters.next()?;
                        if !ESCAPED_IN_DOUBLE_QUOTES.contains(&escaped) {
                            plain.push(ESCAPE);
                        }
                        plain.push(escaped);
                    }
                    quoted => plain.push(quoted),
                }
            },
            ESCAPE => plain.push(characters.next()?),
            bare if bare.is_whitespace() || ENDS_A_WORD.contains(&bare) => return None,
            bare => plain.push(bare),
        }
    }
    (!plain.is_empty()).then_some(plain)
}

#[cfg(test)]
#[path = "../../../tests/dot-shell/script.rs"]
mod tests;
