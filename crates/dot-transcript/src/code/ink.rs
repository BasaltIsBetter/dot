use std::ops::Range;

use crate::ink::Ink;

const NAME: Ink = Ink::Full;
const KEYWORD: Ink = Ink::Secondary;
const PUNCTUATION: Ink = Ink::Receding;
const ESCAPE: char = '\\';
const UNDERSCORE: char = '_';
const EXTENSION_MARK: char = '.';

const RUST: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];
const SCRIPT: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "of",
    "private",
    "protected",
    "public",
    "readonly",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "yield",
];
const PYTHON: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
    "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "self", "True", "try",
    "while", "with", "yield",
];
const GO: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "false",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "nil",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "true",
    "type",
    "var",
];
const C_FAMILY: &[&str] = &[
    "abstract",
    "boolean",
    "break",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "fun",
    "func",
    "guard",
    "if",
    "implements",
    "import",
    "int",
    "interface",
    "let",
    "long",
    "new",
    "nil",
    "null",
    "override",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "self",
    "short",
    "static",
    "struct",
    "super",
    "switch",
    "this",
    "throw",
    "throws",
    "true",
    "try",
    "val",
    "var",
    "void",
    "while",
];
const SHELL: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in",
    "local", "return", "then", "while",
];
const CONFIG: &[&str] = &["false", "true"];

/// The language of a file, told by its extension: which words are its keywords, how a comment
/// starts, which quotes open a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// Rust (`.rs`).
    Rust,
    /// JavaScript and TypeScript (`.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`).
    Script,
    /// Python (`.py`).
    Python,
    /// Go (`.go`).
    Go,
    /// C and its kin: C, C++, C#, Java, Kotlin, Swift, Objective-C.
    CFamily,
    /// Shell scripts (`.sh`, `.bash`, `.zsh`).
    Shell,
    /// Settings files (`.toml`, `.yaml`, `.yml`).
    Config,
    /// Anything else: no keyword, no comment.
    Plain,
}

impl Language {
    /// Returns the language of the file at `path`, by its extension, whatever its case.
    #[must_use]
    pub fn of(path: &str) -> Self {
        let name = path.rsplit('/').next().unwrap_or(path);
        let extension = name
            .rsplit_once(EXTENSION_MARK)
            .map_or("", |(_, extension)| extension)
            .to_ascii_lowercase();
        match extension.as_str() {
            "rs" => Self::Rust,
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => Self::Script,
            "py" => Self::Python,
            "go" => Self::Go,
            "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "java" | "kt" | "kts" | "swift" | "m"
            | "mm" => Self::CFamily,
            "sh" | "bash" | "zsh" => Self::Shell,
            "toml" | "yaml" | "yml" => Self::Config,
            _ => Self::Plain,
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Rust => RUST,
            Self::Script => SCRIPT,
            Self::Python => PYTHON,
            Self::Go => GO,
            Self::CFamily => C_FAMILY,
            Self::Shell => SHELL,
            Self::Config => CONFIG,
            Self::Plain => &[],
        }
    }

    const fn comment(self) -> Option<&'static str> {
        match self {
            Self::Rust | Self::Script | Self::Go | Self::CFamily => Some("//"),
            Self::Python | Self::Shell | Self::Config => Some("#"),
            Self::Plain => None,
        }
    }

    const fn quotes(self) -> &'static [char] {
        match self {
            Self::Rust | Self::Go | Self::CFamily => &['"'],
            Self::Script => &['"', '\'', '`'],
            Self::Python | Self::Shell | Self::Config => &['"', '\''],
            Self::Plain => &[],
        }
    }
}

pub(crate) fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut tokens = Vec::new();
    let mut characters = text.char_indices().peekable();
    while let Some((start, character)) = characters.next() {
        let mut end = start + character.len_utf8();
        let class = class_of(character);
        if class != Class::Other {
            while let Some(&(at, next)) = characters.peek() {
                if class_of(next) != class {
                    break;
                }
                end = at + next.len_utf8();
                characters.next();
            }
        }
        tokens.push(start..end);
    }
    tokens
}

pub(crate) fn inked(text: &str, language: Language) -> Vec<(Range<usize>, Ink)> {
    let mut pieces: Vec<(Range<usize>, Ink)> = Vec::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for token in tokens(text) {
        let piece = &text[token.clone()];
        let outside = quote.is_none();
        if outside
            && let Some(marker) = language.comment()
            && text[token.start..].starts_with(marker)
        {
            push(&mut pieces, token.start..text.len(), PUNCTUATION);
            break;
        }
        let ink = match pieces.last() {
            Some((_, previous)) if piece.starts_with(char::is_whitespace) => *previous,
            _ => ink_of(piece, outside, language),
        };
        follow_quotes(piece, language, &mut quote, &mut escaped);
        push(&mut pieces, token, ink);
    }
    pieces
}

fn ink_of(piece: &str, outside: bool, language: Language) -> Ink {
    match piece.chars().next().map(class_of) {
        Some(Class::Word) if outside && language.keywords().contains(&piece) => KEYWORD,
        Some(Class::Word | Class::Space) | None => NAME,
        Some(Class::Other) => PUNCTUATION,
    }
}

fn follow_quotes(piece: &str, language: Language, quote: &mut Option<char>, escaped: &mut bool) {
    for character in piece.chars() {
        if *escaped {
            *escaped = false;
            continue;
        }
        match *quote {
            Some(_) if character == ESCAPE => *escaped = true,
            Some(open) if character == open => *quote = None,
            None if language.quotes().contains(&character) => *quote = Some(character),
            _ => {}
        }
    }
}

fn push(pieces: &mut Vec<(Range<usize>, Ink)>, range: Range<usize>, ink: Ink) {
    match pieces.last_mut() {
        Some((last, last_ink)) if *last_ink == ink && last.end == range.start => {
            last.end = range.end;
        }
        _ => pieces.push((range, ink)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Other,
}

fn class_of(character: char) -> Class {
    if character.is_alphanumeric() || character == UNDERSCORE {
        Class::Word
    } else if character.is_whitespace() {
        Class::Space
    } else {
        Class::Other
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/code/ink.rs"]
mod tests;
