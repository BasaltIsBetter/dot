use std::time::Duration;

use super::*;
use crate::app::tests::IdTranslator;
use dot_transcript::{Agents, Tool};

fn texts(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.text.as_str()).collect())
        .collect()
}

fn tool(glyph: Glyph, label: &str) -> Tool {
    Tool {
        glyph,
        label: label.to_owned(),
    }
}

#[test]
fn shows_the_agent_its_task_its_actions_what_it_said_and_its_report() {
    let now = Instant::now();
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "find the grid".to_owned(), now);
    agents.name("toolu_1", "Explore".to_owned());
    agents.start_action("toolu_1", "a".to_owned(), tool(Glyph::Read, "layout.rs"));
    agents.end_action("toolu_1", "a", "212 l.".to_owned());
    agents.start_action("toolu_1", "b".to_owned(), tool(Glyph::Search, "« grid »"));
    agents.said("toolu_1", "Looking further.".to_owned());

    let working = agent_lines(
        agents.get("toolu_1").unwrap(),
        TextWidth::new(40),
        now,
        &IdTranslator,
    );
    agents.report("toolu_1", "The grid is computed once.".to_owned(), 10, now);
    agents.end("toolu_1", now + Duration::from_millis(7_800));
    let done = agent_lines(
        agents.get("toolu_1").unwrap(),
        TextWidth::new(40),
        now,
        &IdTranslator,
    );

    assert_eq!(
        texts(&working),
        [
            "⧉ Explore",
            "agent-started-by-claude find the grid",
            "",
            "▤ layout.rs  212 l.  ✓",
            "⌕ « grid »",
            "",
            "Looking further.",
            "",
            "› agent-view-placeholder",
        ]
    );
    assert_eq!(working[4].spans[0].ink, Ink::Live);
    assert_eq!(working[3].spans[0].ink, Ink::Full);
    assert_eq!(
        texts(&done)[7..],
        [
            "",
            "The grid is computed once.",
            "",
            "✓ Explore · agent-read · 7decimal-separator8 unit-seconds",
            "",
            "› agent-view-placeholder",
        ]
    );
}

#[test]
fn wraps_what_the_agent_says_to_the_column() {
    let now = Instant::now();
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "x".to_owned(), now);
    agents.said("toolu_1", "one two three four five".to_owned());

    let lines = agent_lines(
        agents.get("toolu_1").unwrap(),
        TextWidth::new(10),
        now,
        &IdTranslator,
    );

    let said: Vec<String> = texts(&lines)[4..lines.len() - 2].to_vec();
    assert!(said.len() > 1, "{said:?}");
    assert!(said.iter().all(|text| text.chars().count() <= 10));
    assert!(
        lines[4..lines.len() - 2]
            .iter()
            .all(|line| line.indent == TEXT_INDENT)
    );
}

#[test]
fn cuts_a_long_task_and_a_long_action_to_the_width_keeping_the_result_whole() {
    let now = Instant::now();
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "find ".repeat(40), now);
    agents.name("toolu_1", "Explore".to_owned());
    agents.start_action(
        "toolu_1",
        "a".to_owned(),
        tool(Glyph::Read, &"a/".repeat(60)),
    );
    agents.end_action("toolu_1", "a", "212 l.".to_owned());
    agents.start_action(
        "toolu_1",
        "b".to_owned(),
        tool(Glyph::Search, &"b".repeat(80)),
    );

    let lines = texts(&agent_lines(
        agents.get("toolu_1").unwrap(),
        TextWidth::new(40),
        now,
        &IdTranslator,
    ));

    assert!(
        lines[1].ends_with('…') && lines[1].chars().count() == 40,
        "{:?}",
        lines[1]
    );
    assert!(lines[3].ends_with("…  212 l.  ✓"), "{:?}", lines[3]);
    assert_eq!(lines[3].chars().count(), 40);
    assert!(
        lines[4].ends_with('…') && lines[4].chars().count() == 40,
        "{:?}",
        lines[4]
    );
}
