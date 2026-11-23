use std::time::Duration;

use dot_api::{Detail, Text, ToolEnd, ToolKind, ToolUse, Translator};

use crate::status_bar::seconds;
use dot_transcript::{Glyph, NEW_LINE, Tool, normalized, truncated};

const SUBJECT_MOST_CHARACTERS: usize = 240;
const RESULT_GAP: &str = "  ";
const DURATION_GAP: &str = " · ";
const COMMAND_MARK: &str = "⌘ ";
const DONE: &str = "✓";
const FAILED_EXIT: &str = "✗";
const FOUND: &str = "×";
const WEB_SCHEMES: [&str; 2] = ["https://", "http://"];
const PATH_SEPARATOR: char = '/';
const SHOWN_INSTEAD: char = '\u{fffd}';
const INVISIBLE: [(char, char); 10] = [
    ('\u{00ad}', '\u{00ad}'),
    ('\u{061c}', '\u{061c}'),
    ('\u{180e}', '\u{180e}'),
    ('\u{200b}', '\u{200f}'),
    ('\u{2028}', '\u{202e}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206f}'),
    ('\u{feff}', '\u{feff}'),
    ('\u{fff9}', '\u{fffb}'),
    ('\u{e0000}', '\u{e007f}'),
];

pub(super) fn started(tool: &ToolUse, translator: &impl Translator) -> Tool {
    let subject = first_line(&tool.subject);
    let label = match tool.kind {
        ToolKind::Read | ToolKind::Edit | ToolKind::Write => file_name(&subject).to_owned(),
        ToolKind::FindFiles | ToolKind::SearchText => format!(
            "{}{subject}{}",
            translator.translate(Text::QuoteOpen),
            translator.translate(Text::QuoteClose)
        ),
        ToolKind::Fetch => without_scheme(&subject).to_owned(),
        ToolKind::Run | ToolKind::SearchWeb | ToolKind::Delegate | ToolKind::Other => subject,
    };
    Tool {
        glyph: glyph_of(tool.kind),
        label,
    }
}

pub(super) fn result(end: ToolEnd, translator: &impl Translator) -> String {
    let failure = match end {
        ToolEnd::Done(detail) => return measured(detail, translator),
        ToolEnd::Failed => Text::ToolFailed,
        ToolEnd::Refused => Text::ToolRefused,
        ToolEnd::Stopped => Text::ToolStopped,
    };
    translator.translate(failure).into_owned()
}

pub(super) fn aside(
    tool: &Tool,
    result: &str,
    elapsed: Duration,
    translator: &impl Translator,
) -> Option<String> {
    (tool.glyph == Glyph::Command).then(|| {
        let elapsed = seconds(elapsed, translator);
        format!(
            "{COMMAND_MARK}{}{RESULT_GAP}{result}{DURATION_GAP}{elapsed}",
            tool.label
        )
    })
}

pub(in crate::conversation::flow) fn revealed(text: &str) -> String {
    normalized(text, usize::MAX)
        .chars()
        .map(|character| {
            let hidden = INVISIBLE
                .iter()
                .any(|&(first, last)| (first..=last).contains(&character));
            if hidden { SHOWN_INSTEAD } else { character }
        })
        .collect()
}

const fn glyph_of(kind: ToolKind) -> Glyph {
    match kind {
        ToolKind::Read => Glyph::Read,
        ToolKind::FindFiles | ToolKind::SearchText => Glyph::Search,
        ToolKind::Edit => Glyph::Edit,
        ToolKind::Write => Glyph::Create,
        ToolKind::Run => Glyph::Command,
        ToolKind::Delegate => Glyph::Agents,
        ToolKind::Fetch | ToolKind::SearchWeb => Glyph::Web,
        ToolKind::Other => Glyph::Other,
    }
}

fn measured(detail: Detail, translator: &impl Translator) -> String {
    match detail {
        Detail::Nothing | Detail::Exit(0) => DONE.to_owned(),
        Detail::Exit(code) => format!("{FAILED_EXIT} {code}"),
        Detail::Changes { added, removed } => format!("+{added} −{removed}"),
        Detail::Lines(count) => format!("{count} {}", translator.translate(Text::GutterLines)),
        Detail::Files(count) | Detail::Matches(count) => format!("{FOUND}{count}"),
    }
}

fn first_line(subject: &str) -> String {
    let subject = revealed(subject);
    let first = subject.split(NEW_LINE).next().unwrap_or_default();
    truncated(first, SUBJECT_MOST_CHARACTERS)
}

fn file_name(path: &str) -> &str {
    path.trim_end_matches(PATH_SEPARATOR)
        .rsplit(PATH_SEPARATOR)
        .next()
        .unwrap_or(path)
}

fn without_scheme(address: &str) -> &str {
    WEB_SCHEMES
        .iter()
        .find_map(|scheme| address.strip_prefix(scheme))
        .unwrap_or(address)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/reply/tool.rs"]
mod tests;
