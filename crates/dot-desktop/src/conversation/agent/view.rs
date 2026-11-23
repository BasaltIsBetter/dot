use std::time::Instant;

use dot_api::{Text, Translator};

use crate::conversation::flow::{name_of, summary};
use dot_transcript::{Agent, Glyph, Ink, Line, MARKER, Span, TEXT_INDENT, TextWidth, wrap};

const DONE: &str = "  ✓";
const GAP: &str = "  ";

pub(crate) fn agent_lines(
    agent: &Agent,
    width: TextWidth,
    now: Instant,
    translator: &impl Translator,
) -> Vec<Line> {
    let mut lines = vec![
        Line::new(
            0,
            vec![Span::new(
                format!("{} {}", Glyph::Agents.mark(), name_of(agent)),
                Ink::Full,
            )],
        ),
        Line::new(
            TEXT_INDENT,
            vec![Span::new(
                translator.with(Text::AgentStartedByClaude, &agent.mission),
                Ink::Secondary,
            )],
        ),
        blank(),
    ];
    for action in &agent.actions {
        let mut text = format!("{} {}", action.tool.glyph.mark(), action.tool.label);
        let line = match &action.result {
            Some(result) => {
                text.push_str(GAP);
                text.push_str(result);
                text.push_str(DONE);
                Span::new(text, Ink::Full)
            }
            None => Span::new(text, Ink::Live),
        };
        lines.push(Line::new(TEXT_INDENT, vec![line]));
    }
    for said in agent.said.iter().chain(&agent.report) {
        lines.push(blank());
        lines.extend(paragraph(said, width, Ink::Full));
    }
    if agent.ended.is_some() {
        lines.push(blank());
        let summed = format!("✓ {}", summary(agent, now, translator));
        lines.push(Line::new(
            TEXT_INDENT,
            vec![Span::new(summed, Ink::Secondary)],
        ));
    }
    lines.push(blank());
    lines.push(Line::new(
        0,
        vec![
            Span::new(MARKER, Ink::Receding),
            Span::new(
                translator.translate(Text::AgentViewPlaceholder),
                Ink::Secondary,
            ),
        ],
    ));
    lines
}

fn paragraph(text: &str, width: TextWidth, ink: Ink) -> Vec<Line> {
    wrap(text, width.columns())
        .into_iter()
        .map(|range| Line::new(TEXT_INDENT, vec![Span::new(text[range].trim_end(), ink)]))
        .collect()
}

fn blank() -> Line {
    Line::new(0, Vec::new())
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/agent/view.rs"]
mod tests;
