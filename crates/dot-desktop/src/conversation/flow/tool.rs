use dot_api::{Detail, Text, ToolEnd, ToolUse, Translator};

use crate::conversation::wrap::normalized;
use crate::status_bar::truncated;

const SUBJECT_MOST_CHARACTERS: usize = 240;
const DETAIL_GAP: &str = "  ";
const MORE: &str = " …";
const NEW_LINE: char = '\n';
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

pub(super) fn started(tool: &ToolUse, translator: &impl Translator) -> String {
    let kind = translator.translate(tool.kind.text());
    let subject = revealed(&tool.subject);
    let mut lines = subject.split(NEW_LINE);
    let first = lines.next().unwrap_or_default();
    let cut = truncated(first, SUBJECT_MOST_CHARACTERS);
    let more = if lines.next().is_some() && cut == first {
        MORE
    } else {
        ""
    };
    format!("{kind} {cut}{more}")
}

pub(super) fn ended(end: ToolEnd, translator: &impl Translator) -> String {
    let detail = match end {
        ToolEnd::Done(detail) => return measured(detail, translator),
        ToolEnd::Failed => Text::ToolFailed,
        ToolEnd::Refused => Text::ToolRefused,
        ToolEnd::Stopped => Text::ToolStopped,
    };
    format!("{DETAIL_GAP}{}", translator.translate(detail))
}

pub(super) fn revealed(text: &str) -> String {
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

fn measured(detail: Detail, translator: &impl Translator) -> String {
    let (count, unit) = match detail {
        Detail::Nothing => return String::new(),
        Detail::Changes { added, removed } => return format!("{DETAIL_GAP}+{added} −{removed}"),
        Detail::Exit(code) => {
            let exit = translator.translate(Text::ToolExit);
            return format!("{DETAIL_GAP}{exit} {code}");
        }
        Detail::Lines(count) => (count, Text::ToolLines),
        Detail::Files(count) => (count, Text::ToolFiles),
        Detail::Matches(count) => (count, Text::ToolMatches),
    };
    format!("{DETAIL_GAP}{count} {}", translator.count(unit, count))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/tool.rs"]
mod tests;
