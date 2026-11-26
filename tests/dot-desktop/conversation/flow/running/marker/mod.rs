use dot_api::{AgentEvent, ReplyEvent, ToolKind};
use winit::dpi::PhysicalPosition;

use super::*;
use crate::canvas::{Pane, Rectangle, Scene, TextSize};
use crate::conversation::flow::running::tests::start;
use crate::conversation::flow::tests::{
    IdTranslator, after, layout, scene_at, sent, stage, text_of, type_text,
};
use crate::event::Wheel;
use dot_transcript::{Ink, Placed, columns};

const LONG_REPLY: usize = 120;
const WHEEL_LINES_UP: f32 = 40.0;
const OUT_OF_SIGHT: u64 = 500;
const SETTLED: u64 = 1_000;
const MARKER_TEXT: &str = "agents-out-of-sight · Explore, run the tests";

fn agents_under(text: &str, missions: [&str; 2], now: Instant) -> Conversation {
    let mut conversation = sent("explore it", now);
    conversation.hear(ReplyEvent::Text(text.to_owned()), now, &IdTranslator);
    start(
        &mut conversation,
        "toolu_1",
        ToolKind::Delegate,
        missions[0],
        now,
    );
    start(
        &mut conversation,
        "toolu_2",
        ToolKind::Delegate,
        missions[1],
        now,
    );
    let named = ReplyEvent::Agent {
        id: "toolu_1".to_owned(),
        event: AgentEvent::Started {
            kind: "Explore".to_owned(),
        },
    };
    conversation.hear(named, now, &IdTranslator);
    conversation
}

fn scrolled_up(mut conversation: Conversation, now: Instant) -> Conversation {
    scene_at(&mut conversation, now);
    let pointer = PhysicalPosition::new(100.0, 100.0);
    let wheel = Wheel::Lines(WHEEL_LINES_UP);
    conversation.scroll(wheel, (pointer, &layout()), now, &IdTranslator);
    conversation
}

fn below_the_window(now: Instant) -> Conversation {
    let reply = "a line\n".repeat(LONG_REPLY);
    scrolled_up(
        agents_under(&reply, ["find the grid", "run the tests"], now),
        now,
    )
}

fn settled_scene(conversation: &mut Conversation, now: Instant) -> Scene {
    scene_at(conversation, now);
    scene_at(conversation, after(now, OUT_OF_SIGHT));
    scene_at(conversation, after(now, SETTLED))
}

fn placed_with<'a>(scene: &'a Scene, text: &str) -> Option<(&'a Pane, &'a Placed)> {
    scene.panes.iter().find_map(|pane| {
        pane.lines
            .iter()
            .find(|placed| text_of(placed) == text)
            .map(|placed| (pane, placed))
    })
}

fn right_of(pane: &Pane, text: &str) -> f32 {
    let advance = layout().advance_of(TextSize::Small);
    pane.left + f32::from(u16::try_from(columns(text)).unwrap()) * advance
}

fn marker_middle(scene: &Scene) -> [f32; 2] {
    let (arrow, placed) = placed_with(scene, "↓").unwrap();
    let (text, _) = placed_with(scene, MARKER_TEXT).unwrap();
    [
        f32::midpoint(arrow.left, right_of(text, MARKER_TEXT)),
        placed.top + 2.0,
    ]
}

fn press_at(conversation: &mut Conversation, at: [f32; 2], now: Instant) -> bool {
    conversation.press_marker(at, &layout(), now, &IdTranslator)
}

#[test]
fn shows_nothing_while_the_lanes_are_in_sight() {
    let now = Instant::now();
    let mut conversation = agents_under("Exploring.", ["find the grid", "run the tests"], now);

    let scene = settled_scene(&mut conversation, now);

    assert!(placed_with(&scene, "↓").is_none());
    assert!(placed_with(&scene, "↑").is_none());
    assert!(placed_with(&scene, MARKER_TEXT).is_none());
}

#[test]
fn points_down_to_the_lanes_below_the_window_and_names_their_agents() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);

    let scene = settled_scene(&mut conversation, now);

    let stage = stage();
    let layout = layout();
    let (arrow, placed) = placed_with(&scene, "↓").unwrap();
    let (text, words) = placed_with(&scene, MARKER_TEXT).unwrap();
    let top = (stage.height - stage.points(64.0) - stage.row).round();
    assert_eq!(placed.top, layout.text_top(TextSize::Small, top, stage.row));
    assert_eq!(words.top, placed.top);
    assert_eq!(placed.line.spans[0].ink, Ink::Secondary);
    assert_eq!(placed.line.alpha, u8::MAX);
    let middle = f32::midpoint(stage.column[0], stage.column[1]);
    assert!((f32::midpoint(arrow.left, right_of(text, MARKER_TEXT)) - middle).abs() <= 1.0);
    let figure: Vec<&Rectangle> = scene
        .marks
        .iter()
        .filter(|mark| mark.left > arrow.left && mark.left < text.left)
        .filter(|mark| mark.top >= top && mark.top + mark.height <= top + stage.row)
        .collect();
    assert_ne!(figure.len(), 0);
    assert!(figure.iter().all(|mark| mark.color.ink == Ink::Live));
    let gap = stage.points(6.0);
    let advance = layout.advance_of(TextSize::Small);
    let figure_left = figure.iter().map(|mark| mark.left).fold(f32::MAX, f32::min);
    assert!(
        (figure_left - (arrow.left + advance + gap)).abs() <= 1.0,
        "{figure_left}"
    );
}

#[test]
fn points_up_to_the_lanes_above_the_window() {
    let now = Instant::now();
    let mut conversation = agents_under("Exploring.", ["find the grid", "run the tests"], now);
    type_text(&mut conversation, &"a word\n".repeat(LONG_REPLY), now);

    let scene = settled_scene(&mut conversation, now);

    let stage = stage();
    let (_, placed) = placed_with(&scene, "↑").unwrap();
    let top = stage.points(16.0);
    assert_eq!(
        placed.top,
        layout().text_top(TextSize::Small, top, stage.row)
    );
    assert!(placed_with(&scene, MARKER_TEXT).is_some());
    assert!(placed_with(&scene, "↓").is_none());
}

#[test]
fn fades_in_then_out_once_the_lanes_come_back_in_sight() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);
    scene_at(&mut conversation, now);
    scene_at(&mut conversation, after(now, OUT_OF_SIGHT));

    let appearing = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 100));
    let shown = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 600));
    conversation.reader.come_back();
    scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 700));
    scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 1_200));
    let leaving = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 1_300));
    let gone = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 1_600));

    let alpha = |scene: &Scene| placed_with(scene, "↓").map(|(_, placed)| placed.line.alpha);
    assert!(alpha(&appearing).is_some_and(|alpha| alpha > 0 && alpha < u8::MAX));
    assert_eq!(alpha(&shown), Some(u8::MAX));
    assert!(alpha(&leaving).is_some_and(|alpha| alpha < u8::MAX));
    assert_eq!(alpha(&gone), None);
    assert!(
        conversation
            .marker
            .is_fading(after(now, OUT_OF_SIGHT + 1_300))
    );
    assert!(
        !conversation
            .marker
            .is_fading(after(now, OUT_OF_SIGHT + 1_600))
    );
}

#[test]
fn lights_up_under_the_pointer() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);
    let middle = marker_middle(&settled_scene(&mut conversation, now));

    conversation.point(Some(middle));
    let hovered = scene_at(&mut conversation, after(now, SETTLED));

    let (_, placed) = placed_with(&hovered, "↓").unwrap();
    assert_eq!(placed.line.spans[0].ink, Ink::Full);
}

#[test]
fn a_click_on_it_brings_the_first_lane_out_of_sight_to_the_reading_line() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);
    let middle = marker_middle(&settled_scene(&mut conversation, now));
    let lane = conversation.marker.last.as_ref().unwrap().row;

    let jumped = press_at(&mut conversation, middle, after(now, SETTLED));
    scene_at(&mut conversation, after(now, 2 * SETTLED));
    scene_at(&mut conversation, after(now, 2 * SETTLED + OUT_OF_SIGHT));
    let scene = scene_at(&mut conversation, after(now, 3 * SETTLED));

    assert!(jumped);
    let focus = conversation.reader.focus(after(now, 3 * SETTLED));
    assert!((focus - rows(lane)).abs() < 1.0, "{focus} {lane}");
    assert!(placed_with(&scene, MARKER_TEXT).is_none());
    let (_, lane) = placed_with(&scene, "Explore  ↗").unwrap();
    assert!(lane.top >= 0.0 && lane.top < stage().floor, "{}", lane.top);
}

#[test]
fn a_click_beside_it_or_while_it_leaves_does_nothing() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);
    let [x, y] = marker_middle(&settled_scene(&mut conversation, now));
    let focus = conversation.reader.focus(after(now, SETTLED));

    let beside = press_at(&mut conversation, [x, y - stage().row], after(now, SETTLED));
    scene_at(&mut conversation, after(now, 2 * SETTLED));
    let kept = conversation.reader.focus(after(now, 2 * SETTLED));
    conversation.reader.come_back();
    scene_at(&mut conversation, after(now, 2 * SETTLED));
    scene_at(&mut conversation, after(now, 2 * SETTLED + OUT_OF_SIGHT));
    let leaving = press_at(
        &mut conversation,
        [x, y],
        after(now, 2 * SETTLED + OUT_OF_SIGHT),
    );

    assert!(!beside);
    assert!((kept - focus).abs() < f64::EPSILON, "{focus} {kept}");
    assert!(!leaving);
}

#[test]
fn cuts_the_names_to_the_column() {
    let now = Instant::now();
    let long = "a very long agent name ".repeat(10);
    let reply = "a line\n".repeat(LONG_REPLY);
    let mut conversation = scrolled_up(agents_under(&reply, ["find the grid", &long], now), now);

    let scene = settled_scene(&mut conversation, now);

    let (pane, placed) = scene
        .panes
        .iter()
        .flat_map(|pane| pane.lines.iter().map(move |placed| (pane, placed)))
        .find(|(_, placed)| text_of(placed).starts_with("agents-out-of-sight"))
        .unwrap();
    let words = text_of(placed);
    assert!(words.ends_with('…'), "{words}");
    assert!(right_of(pane, &words) <= stage().column[1] + 1.0);
}

#[test]
fn tells_a_lane_above_below_or_in_sight_by_its_row() {
    let stage = stage();
    let margin = stage.points(8.0);
    let lowest = stage.lowest_row_bottom();

    assert_eq!(side_of(margin - 1.0, &stage), Some(Side::Above));
    assert_eq!(side_of(margin, &stage), None);
    assert_eq!(side_of(lowest - stage.row, &stage), None);
    assert_eq!(side_of(lowest - stage.row + 1.0, &stage), Some(Side::Below));
    assert_eq!(side_of(-500.0, &stage), Some(Side::Above));
}

#[test]
fn a_click_on_it_brings_the_lanes_above_down_to_the_reading_line() {
    let now = Instant::now();
    let mut conversation = agents_under("Exploring.", ["find the grid", "run the tests"], now);
    type_text(&mut conversation, &"a word\n".repeat(LONG_REPLY), now);
    let scene = settled_scene(&mut conversation, now);
    let (arrow, placed) = placed_with(&scene, "↑").unwrap();
    let (text, _) = placed_with(&scene, MARKER_TEXT).unwrap();
    let middle = [
        f32::midpoint(arrow.left, right_of(text, MARKER_TEXT)),
        placed.top + 2.0,
    ];
    let lane = conversation.marker.last.as_ref().unwrap().row;
    let first = conversation.below(after(now, SETTLED)).running.unwrap();

    let jumped = press_at(&mut conversation, middle, after(now, SETTLED));
    scene_at(&mut conversation, after(now, 2 * SETTLED));
    scene_at(&mut conversation, after(now, 3 * SETTLED));

    assert!(jumped);
    assert_eq!(lane, first + 2);
    let focus = conversation.reader.focus(after(now, 3 * SETTLED));
    assert!((focus - rows(lane)).abs() < 0.01, "{focus} {lane}");
}

#[test]
fn fades_what_passes_behind_it_as_it_appears_and_centres_its_figure() {
    let now = Instant::now();
    let wide = "x".repeat(80);
    let reply = format!("{wide}\n").repeat(LONG_REPLY);
    let lanes = agents_under(&reply, ["find the grid", "run the tests"], now);
    let mut conversation = scrolled_up(lanes, now);
    scene_at(&mut conversation, now);
    scene_at(&mut conversation, after(now, OUT_OF_SIGHT));

    let appearing = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 100));

    let (_, placed) = placed_with(&appearing, "↓").unwrap();
    let shown = placed.line.alpha;
    let stage = stage();
    let behind = appearing.panes[0]
        .lines
        .iter()
        .find(|line| (line.top - placed.top).abs() < stage.row / 2.0 && text_of(line) == wide)
        .unwrap();
    assert!(shown > 0 && shown < u8::MAX);
    assert!(
        behind.line.alpha.abs_diff(u8::MAX - shown) <= 2,
        "{} {shown}",
        behind.line.alpha
    );
    let top = (stage.height - stage.points(64.0) - stage.row).round();
    let (arrow, _) = placed_with(&appearing, "↓").unwrap();
    let (text, _) = placed_with(&appearing, MARKER_TEXT).unwrap();
    let figure: Vec<&Rectangle> = appearing
        .marks
        .iter()
        .filter(|mark| mark.left > arrow.left && mark.left < text.left)
        .filter(|mark| mark.top >= top - 1.0 && mark.top < top + stage.row)
        .collect();
    let highest = figure.iter().map(|mark| mark.top).fold(f32::MAX, f32::min);
    let lowest = figure
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    let centre = top + stage.row / 2.0;
    assert!(
        (f32::midpoint(highest, lowest) - centre).abs() <= 1.0,
        "{highest} {lowest} {centre}"
    );
}

#[test]
fn keeps_painting_while_it_fades_in() {
    let now = Instant::now();
    let mut conversation = below_the_window(now);
    scene_at(&mut conversation, now);
    scene_at(&mut conversation, after(now, OUT_OF_SIGHT));

    let fading = scene_at(&mut conversation, after(now, OUT_OF_SIGHT + 100));

    assert!(fading.animating);
    assert!(
        conversation
            .marker
            .is_fading(after(now, OUT_OF_SIGHT + 100))
    );
}
