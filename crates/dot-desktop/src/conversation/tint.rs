use dot_transcript::{Ink, Line, Span};

use crate::canvas::Pane;

const TOOL_GLYPHS: [char; 7] = ['▤', '⌕', '✎', '✚', '⌘', '⧉', '◎'];
const PASSED: char = '✓';
const FAILED: char = '✗';
const SPACE: char = ' ';
pub(in crate::conversation) const GOOD: [u8; 3] = [0x4a, 0xde, 0x80];
const BAD: [u8; 3] = [0xff, 0x5c, 0x5c];
const WARN: [u8; 3] = [0xfb, 0xbf, 0x24];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Colours {
    pub(crate) tool: Option<[u8; 3]>,
    pub(crate) states: bool,
}

pub(crate) struct Tints<'a> {
    pub(crate) tool: Option<[u8; 3]>,
    pub(crate) states: bool,
    pub(crate) stopped: &'a str,
}

pub(crate) fn tint(panes: &mut [Pane], tints: &Tints<'_>) {
    if tints.tool.is_none() && !tints.states {
        return;
    }
    for pane in panes {
        for placed in &mut pane.lines {
            if may_tint(&placed.line, tints) {
                tint_line(&mut placed.line, tints);
            }
        }
    }
}

fn may_tint(line: &Line, tints: &Tints<'_>) -> bool {
    line.spans.iter().any(|span| {
        let tool = tints.tool.is_some() && span.text.contains(TOOL_GLYPHS);
        let state = tints.states
            && (span.text.contains([PASSED, FAILED])
                || (!tints.stopped.is_empty() && span.text.contains(tints.stopped)));
        tool || state
    })
}

fn tint_line(line: &mut Line, tints: &Tints<'_>) {
    let characters: Vec<char> = line
        .spans
        .iter()
        .flat_map(|span| span.text.chars())
        .collect();
    let mut colours = vec![None; characters.len()];
    if let Some(tool) = tints.tool {
        mark_tools(&characters, &mut colours, tool);
    }
    if tints.states {
        mark_states(&characters, &mut colours, tints.stopped);
    }
    if colours.iter().all(Option::is_none) {
        return;
    }
    line.spans = split(std::mem::take(&mut line.spans), &colours);
}

fn mark_tools(characters: &[char], colours: &mut [Option<[u8; 3]>], tool: [u8; 3]) {
    if let Some(first) = characters.iter().position(|&character| character != SPACE)
        && TOOL_GLYPHS.contains(&characters[first])
    {
        colours[first] = Some(tool);
    }
}

fn mark_states(characters: &[char], colours: &mut [Option<[u8; 3]>], stopped: &str) {
    for (at, &character) in characters.iter().enumerate() {
        let colour = match character {
            PASSED => GOOD,
            FAILED => BAD,
            _ => continue,
        };
        let start = counted_from(characters, at);
        for slot in &mut colours[start..=at] {
            *slot = Some(colour);
        }
    }
    let word: Vec<char> = stopped.chars().collect();
    if word.is_empty() {
        return;
    }
    for start in 0..characters.len() {
        if characters[start..].starts_with(&word) {
            for slot in &mut colours[start..start + word.len()] {
                *slot = Some(WARN);
            }
        }
    }
}

fn counted_from(characters: &[char], mark: usize) -> usize {
    let Some(space) = mark
        .checked_sub(1)
        .filter(|&space| characters[space] == SPACE)
    else {
        return mark;
    };
    let digits = characters[..space]
        .iter()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits == 0 { mark } else { space - digits }
}

fn split(spans: Vec<Span>, colours: &[Option<[u8; 3]>]) -> Vec<Span> {
    let mut tinted = Vec::with_capacity(spans.len() + 2);
    let mut offset = 0;
    for span in spans {
        let count = span.text.chars().count();
        let own = &colours[offset..offset + count];
        offset += count;
        if !is_grey(span.ink) || own.iter().all(Option::is_none) {
            tinted.push(span);
            continue;
        }
        let mut run = String::new();
        let mut run_colour = own.first().copied().flatten();
        for (character, &colour) in span.text.chars().zip(own) {
            if colour != run_colour && !run.is_empty() {
                tinted.push(piece(std::mem::take(&mut run), run_colour, &span));
            }
            run_colour = colour;
            run.push(character);
        }
        if !run.is_empty() {
            tinted.push(piece(run, run_colour, &span));
        }
    }
    tinted
}

fn piece(text: String, colour: Option<[u8; 3]>, span: &Span) -> Span {
    let ink = colour.map_or(span.ink, Ink::Rgb);
    Span::new(text, ink).heated(span.heat)
}

const fn is_grey(ink: Ink) -> bool {
    !matches!(ink, Ink::Live | Ink::Rgb(_))
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/tint.rs"]
mod tests;
