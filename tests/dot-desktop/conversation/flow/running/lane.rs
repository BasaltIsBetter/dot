use std::time::Duration;

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::tests::{layout, stage};
use dot_transcript::{Agents, Glyph, Tool};

fn agent_at_work(now: Instant) -> Agents {
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "find the grid".to_owned(), now);
    agents.name("toolu_1", "Explore".to_owned());
    agents
}

fn drawn(agents: &Agents, now: Instant) -> DrawnLane {
    let agent = agents.get("toolu_1").unwrap();
    lane(agent, 500.0, (&stage(), &layout()), now, &IdTranslator)
}

fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

#[test]
fn names_the_agent_then_what_it_does() {
    let now = Instant::now();
    let mut agents = agent_at_work(now);
    let idle = text_of(&drawn(&agents, now).text);
    agents.start_action(
        "toolu_1",
        "a".to_owned(),
        Tool {
            glyph: Glyph::Read,
            label: "layout.rs".to_owned(),
        },
    );

    let reading = text_of(&drawn(&agents, now).text);

    assert_eq!(idle, "── Explore  ↗");
    assert_eq!(reading, "── Explore  ↗ ▤ layout.rs");
}

#[test]
fn pads_a_short_name_to_eight_columns() {
    let now = Instant::now();
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "x".to_owned(), now);
    agents.name("toolu_1", "web".to_owned());

    assert_eq!(text_of(&drawn(&agents, now).text), "── web      ↗");
}

#[test]
fn shows_its_rate_live_at_the_right_of_the_column_after_its_spark() {
    let now = Instant::now();
    let mut agents = agent_at_work(now);
    let silent = drawn(&agents, now);
    agents.wrote("toolu_1", 40, now + Duration::from_millis(100));

    let writing = drawn(&agents, now + Duration::from_millis(500));

    let stage = stage();
    let small = layout().advance_of(TextSize::Small);
    assert_eq!(text_of(&silent.rate), "––– unit-tokens-per-second");
    assert_eq!(text_of(&writing.rate), "40 unit-tokens-per-second");
    assert_eq!(writing.rate.line.spans[0].ink, Ink::Live);
    let rate_left = stage.column[0] + f32::from(writing.rate.line.indent) * small;
    let rate_right = rate_left + f32::from(columns_of(&text_of(&writing.rate))) * small;
    assert!(rate_right <= stage.column[1] + 0.5 && rate_right > stage.column[1] - small);
    assert_eq!(writing.spark.len(), 48);
    let spark_right = writing
        .spark
        .iter()
        .map(|cell| cell.left + cell.width)
        .fold(f32::MIN, f32::max);
    assert!(spark_right < rate_left);
    assert!(
        writing
            .spark
            .iter()
            .all(|cell| cell.top >= 500.0 && cell.top + cell.height <= 500.0 + stage.row)
    );
}

#[test]
fn cuts_what_it_does_before_the_spark() {
    let now = Instant::now();
    let mut agents = agent_at_work(now);
    agents.start_action(
        "toolu_1",
        "a".to_owned(),
        Tool {
            glyph: Glyph::Read,
            label: "a/".repeat(200),
        },
    );

    let lane = drawn(&agents, now);

    let text = text_of(&lane.text);
    assert!(text.ends_with('…'), "{text}");
    let advance = layout().advance_of(TextSize::Regular);
    let text_right = stage().column[0] + f32::from(columns_of(&text)) * advance;
    let spark_left = lane
        .spark
        .iter()
        .map(|cell| cell.left)
        .fold(f32::MAX, f32::min);
    assert!(text_right < spark_left);
}
