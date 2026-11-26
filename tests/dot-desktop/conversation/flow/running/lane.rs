use std::time::Duration;

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::tests::{layout, stage};
use dot_transcript::{Agents, Tool};

const SESSION: [u8; 3] = [0xa9, 0x70, 0xff];

fn agent_at_work(now: Instant) -> Agents {
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "find the grid".to_owned(), now);
    agents.name("toolu_1", "Explore".to_owned());
    agents
}

fn drawn(agents: &Agents, now: Instant) -> DrawnLane {
    drawn_from(agents, 0, now)
}

fn drawn_from(agents: &Agents, indent: u16, now: Instant) -> DrawnLane {
    drawn_in(agents, (indent, None), now)
}

fn drawn_in(agents: &Agents, (indent, tool): (u16, Option<[u8; 3]>), now: Instant) -> DrawnLane {
    let agent = agents.get("toolu_1").unwrap();
    let silhouette = Silhouette::at(layout().scale());
    lane(
        agent,
        (indent, 500.0),
        (&stage(), &layout()),
        (&silhouette, tool),
        now,
        &IdTranslator,
    )
}

fn reading(agents: &mut Agents, glyph: Glyph, label: &str) {
    agents.start_action(
        "toolu_1",
        "a".to_owned(),
        Tool {
            glyph,
            label: label.to_owned(),
        },
    );
}

fn pieces(placed: &Placed) -> Vec<(&str, Ink)> {
    placed
        .line
        .spans
        .iter()
        .map(|span| (span.text.as_str(), span.ink))
        .collect()
}

fn bounds(marks: &[Rectangle]) -> [f32; 4] {
    let left = marks.iter().map(|mark| mark.left).fold(f32::MAX, f32::min);
    let top = marks.iter().map(|mark| mark.top).fold(f32::MAX, f32::min);
    let right = marks
        .iter()
        .map(|mark| mark.left + mark.width)
        .fold(f32::MIN, f32::max);
    let bottom = marks
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    [left, top, right, bottom]
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

    assert_eq!(idle, "Explore  ↗");
    assert_eq!(reading, "Explore  ↗ ▤ layout.rs");
}

#[test]
fn pads_a_short_name_to_eight_columns() {
    let now = Instant::now();
    let mut agents = Agents::default();
    agents.start("toolu_1".to_owned(), "x".to_owned(), now);
    agents.name("toolu_1", "web".to_owned());

    assert_eq!(text_of(&drawn(&agents, now).text), "web      ↗");
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
    let shown = lane.text.line.indent + columns_of(&text);
    let text_right = stage().column[0] + f32::from(shown) * advance;
    let spark_left = lane
        .spark
        .iter()
        .map(|cell| cell.left)
        .fold(f32::MAX, f32::min);
    assert!(text_right < spark_left);
}

#[test]
fn starts_from_its_indent_and_still_ends_before_the_spark() {
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

    let flush = drawn_from(&agents, 0, now);
    let indented = drawn_from(&agents, 6, now);

    let text = text_of(&indented.text);
    assert_eq!(flush.text.line.indent, FIGURE_COLUMNS);
    assert_eq!(indented.text.line.indent, 6 + FIGURE_COLUMNS);
    assert_eq!(columns_of(&text) + 6, columns_of(&text_of(&flush.text)));
    let advance = layout().advance_of(TextSize::Regular);
    let shown = indented.text.line.indent + columns_of(&text);
    let text_right = stage().column[0] + f32::from(shown) * advance;
    let spark_left = indented
        .spark
        .iter()
        .map(|cell| cell.left)
        .fold(f32::MAX, f32::min);
    assert!(text_right < spark_left);
}

#[test]
fn centres_its_spark_in_the_row() {
    let now = Instant::now();
    let agents = agent_at_work(now);

    let lane = drawn(&agents, now);

    let stage = stage();
    let height = stage.points(MINI_SPARK_HEIGHT);
    let expected = (500.0 + (stage.row - height) / 2.0).round();
    let top = lane
        .spark
        .iter()
        .map(|cell| cell.top)
        .fold(f32::MAX, f32::min);
    let bottom = lane
        .spark
        .iter()
        .map(|cell| cell.top + cell.height)
        .fold(f32::MIN, f32::max);
    assert_eq!(top, expected);
    assert!(
        (bottom - top - height).abs() <= 1.0,
        "{top} {bottom} {height}"
    );
}

#[test]
fn draws_the_figure_in_the_three_columns_before_the_name_centred_in_the_row() {
    let now = Instant::now();
    let agents = agent_at_work(now);

    let lane = drawn_from(&agents, 4, now);

    let stage = stage();
    let advance = layout().advance_of(TextSize::Regular);
    let figure_left = stage.column[0] + 4.0 * advance;
    let text_left = stage.column[0] + f32::from(lane.text.line.indent) * advance;
    let [left, top, right, bottom] = bounds(&lane.figure);
    let height = Silhouette::at(layout().scale()).size()[1];
    assert!(
        (left - figure_left.round()).abs() <= 1.0,
        "{left} {figure_left}"
    );
    assert!(right <= text_left, "{right} {text_left}");
    assert!(right - left > 2.0 * advance / 3.0, "{left} {right}");
    assert!(
        top >= 500.0 && bottom <= 500.0 + stage.row,
        "{top} {bottom}"
    );
    let middle = 500.0 + stage.row / 2.0;
    assert!(
        (f32::midpoint(top, bottom) - middle).abs() <= 1.0,
        "{top} {bottom}"
    );
    assert!(
        (bottom - top - height).abs() <= 1.0,
        "{top} {bottom} {height}"
    );
}

#[test]
fn draws_the_figure_live_or_in_the_tool_colour() {
    let now = Instant::now();
    let agents = agent_at_work(now);

    let live = drawn_in(&agents, (0, None), now);
    let tinted = drawn_in(&agents, (0, Some(SESSION)), now);

    assert_ne!(live.figure.len(), 0);
    assert!(live.figure.iter().all(|mark| mark.color.ink == Ink::Live));
    assert!(
        tinted
            .figure
            .iter()
            .all(|mark| mark.color.ink == Ink::Rgb(SESSION))
    );
}

#[test]
fn draws_the_glyph_of_what_it_does_in_the_tool_colour() {
    let now = Instant::now();
    let mut agents = agent_at_work(now);
    reading(&mut agents, Glyph::Read, "layout.rs");

    let tinted = drawn_in(&agents, (0, Some(SESSION)), now);
    let plain = drawn_in(&agents, (0, None), now);

    assert_eq!(
        pieces(&tinted.text),
        [
            ("Explore  ↗ ", Ink::Full),
            ("▤", Ink::Rgb(SESSION)),
            (" layout.rs", Ink::Full),
        ]
    );
    assert_eq!(pieces(&plain.text), [("Explore  ↗ ▤ layout.rs", Ink::Full)]);
}

#[test]
fn leaves_the_glyph_of_another_tool_and_an_unnamed_action_alone() {
    let now = Instant::now();
    let idle = agent_at_work(now);
    let mut other = agent_at_work(now);
    reading(&mut other, Glyph::Other, "mcp");
    let mut bare = agent_at_work(now);
    reading(&mut bare, Glyph::Read, "");

    let idle = drawn_in(&idle, (0, Some(SESSION)), now);
    let other = drawn_in(&other, (0, Some(SESSION)), now);
    let bare = drawn_in(&bare, (0, Some(SESSION)), now);

    assert_eq!(pieces(&idle.text), [("Explore  ↗", Ink::Full)]);
    assert_eq!(pieces(&other.text), [("Explore  ↗ ◇ mcp", Ink::Full)]);
    assert_eq!(
        pieces(&bare.text),
        [
            ("Explore  ↗ ", Ink::Full),
            ("▤", Ink::Rgb(SESSION)),
            (" ", Ink::Full)
        ]
    );
}

#[test]
fn does_not_tint_the_ellipsis_that_cuts_at_the_glyph() {
    let now = Instant::now();
    let named = |length: usize| {
        let mut agents = Agents::default();
        agents.start("toolu_1".to_owned(), "x".to_owned(), now);
        agents.name("toolu_1", "a".repeat(length));
        reading(&mut agents, Glyph::Read, "layout.rs");
        agents
    };
    let room = columns_of(&text_of(&drawn_in(&named(400), (0, None), now).text));

    let cut_before = drawn_in(&named(400), (0, Some(SESSION)), now);
    let cut_at = drawn_in(&named(usize::from(room) - 4), (0, Some(SESSION)), now);

    for lane in [&cut_before, &cut_at] {
        let text = text_of(&lane.text);
        assert!(text.ends_with('…'), "{text}");
        assert!(
            lane.text
                .line
                .spans
                .iter()
                .all(|span| span.ink == Ink::Full)
        );
    }
    assert_eq!(cut_at.text.line.spans.len(), 2);
}
