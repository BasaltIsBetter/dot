use dot_api::{AgentEvent, ReplyEvent, ToolKind, ToolUse};

use super::*;
use crate::canvas::Scene;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, scene_at, sent, stage, text_of,
};

pub(crate) fn start(
    conversation: &mut Conversation,
    id: &str,
    kind: ToolKind,
    subject: &str,
    now: Instant,
) {
    conversation.hear(
        ReplyEvent::ToolStarted {
            id: id.to_owned(),
            tool: ToolUse {
                kind,
                subject: subject.to_owned(),
            },
        },
        now,
        &IdTranslator,
    );
}

pub(crate) fn print(conversation: &mut Conversation, id: &str, lines: &[&str], now: Instant) {
    conversation.hear(
        ReplyEvent::ToolOutput {
            id: id.to_owned(),
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        },
        now,
        &IdTranslator,
    );
}

pub(crate) fn with_agents(now: Instant) -> Conversation {
    let mut conversation = sent("explore it", now);
    start(
        &mut conversation,
        "toolu_1",
        ToolKind::Delegate,
        "find the grid",
        now,
    );
    start(
        &mut conversation,
        "toolu_2",
        ToolKind::Delegate,
        "run the tests",
        now,
    );
    conversation.hear(
        ReplyEvent::Agent {
            id: "toolu_1".to_owned(),
            event: AgentEvent::Started {
                kind: "Explore".to_owned(),
            },
        },
        now,
        &IdTranslator,
    );
    conversation
}

fn small_texts(scene: &Scene) -> Vec<String> {
    scene.panes[1]
        .lines
        .iter()
        .map(text_of)
        .filter(|text| text.starts_with(OUTPUT_INDENT))
        .collect()
}

fn top_of(scene: &Scene, text: &str) -> f32 {
    scene.panes[0]
        .lines
        .iter()
        .find(|placed| text_of(placed) == text)
        .unwrap()
        .top
}

#[test]
fn opens_a_curtain_under_the_answer_while_a_command_runs() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    conversation.hear(
        ReplyEvent::Text("Running the tests.".into()),
        now,
        &IdTranslator,
    );
    start(
        &mut conversation,
        "toolu_1",
        ToolKind::Run,
        "cargo test -p dot-desktop",
        now,
    );

    let before = flow_texts(&scene_at(&mut conversation, after(now, 50)));
    print(
        &mut conversation,
        "toolu_1",
        &["a", "b", "c"],
        after(now, 100),
    );
    let scene = scene_at(&mut conversation, after(now, 200));

    assert!(
        before.contains(&"⌘ cargo test -p dot-desktop".to_owned()),
        "{before:?}"
    );
    let header = top_of(&scene, "⌘ cargo test -p dot-desktop");
    let answer = top_of(&scene, "Running the tests.");
    let draft = top_of(&scene, "›");
    assert!(
        answer < header && header < draft,
        "{answer} {header} {draft}"
    );
    assert_eq!(small_texts(&scene), ["  a", "  b", "  c"]);
}

#[test]
fn shows_the_output_small_and_quiet_one_row_each_under_the_header() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    start(&mut conversation, "toolu_1", ToolKind::Run, "ls", now);
    print(&mut conversation, "toolu_1", &["one", "two"], now);

    let scene = scene_at(&mut conversation, after(now, 100));

    let header = scene.panes[0]
        .lines
        .iter()
        .find(|placed| text_of(placed) == "⌘ ls")
        .unwrap();
    let small: Vec<&Placed> = scene.panes[1].lines.iter().collect();
    assert_eq!(scene.panes[1].size, TextSize::Small);
    let stage = stage();
    assert_eq!(header.line.spans[0].ink, Ink::Live);
    assert!(
        small
            .iter()
            .all(|placed| placed.line.spans[0].ink == Ink::Secondary)
    );
    let below_header = small[0].top - header.top;
    assert!(
        (below_header - stage.row).abs() < stage.row / 2.0,
        "{below_header}"
    );
    assert!((small[1].top - small[0].top - stage.row).abs() < 0.5);
}

#[test]
fn shows_what_a_command_prints_as_it_is_seen_and_cut_to_the_column() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    start(&mut conversation, "toolu_1", ToolKind::Run, "cat", now);
    let long = "x".repeat(400);
    print(&mut conversation, "toolu_1", &["a\u{202e}b", &long], now);

    let texts = small_texts(&scene_at(&mut conversation, after(now, 100)));

    assert_eq!(texts[0], "  a\u{fffd}b");
    assert!(texts[1].ends_with('…'));
    assert!(texts[1].chars().count() < 400);
}

#[test]
fn heads_the_agents_at_work_then_gives_each_a_lane() {
    let now = Instant::now();
    let mut conversation = with_agents(now);

    let rows: Vec<String> = conversation
        .running_rows()
        .into_iter()
        .map(|row| match row {
            RunningRow::Agents(count) => format!("agents {count}"),
            RunningRow::Lane(id, _) => format!("lane {id}"),
            RunningRow::Command(label) => format!("command {label}"),
            RunningRow::Output(text) => format!("output {text}"),
        })
        .collect();
    let texts = flow_texts(&scene_at(&mut conversation, after(now, 100)));

    assert_eq!(rows, ["agents 2", "lane toolu_1", "lane toolu_2"]);
    assert!(texts.contains(&"⧉ agents-at-work".to_owned()), "{texts:?}");
    assert_eq!(conversation.lane_at(1), Some("toolu_1"));
    assert_eq!(conversation.lane_at(0), None);
    assert_eq!(conversation.lane_at(9), None);
}

#[test]
fn the_band_follows_the_running_rows_and_the_prompt_goes_under_them() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    conversation.hear(ReplyEvent::Text("Running.".into()), now, &IdTranslator);
    let quiet = conversation.below();
    start(
        &mut conversation,
        "toolu_1",
        ToolKind::Run,
        "cargo test",
        now,
    );
    print(&mut conversation, "toolu_1", &["a", "b"], now);

    let busy = conversation.below();
    let plan = conversation.plan(&stage());

    assert_eq!(busy.running, Some(conversation.transcript.end_row()));
    assert_eq!(busy.draft, quiet.draft + 3);
    assert_eq!(plan.writing, busy.running);
}

#[test]
fn the_curtain_closes_once_the_command_ends() {
    let now = Instant::now();
    let mut conversation = sent("test it", now);
    start(&mut conversation, "toolu_1", ToolKind::Run, "ls", now);
    print(&mut conversation, "toolu_1", &["a"], now);

    conversation.hear(
        ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: dot_api::ToolEnd::Done(dot_api::Detail::Exit(0)),
        },
        after(now, 800),
        &IdTranslator,
    );

    assert_eq!(conversation.running_rows().len(), 0);
    assert_eq!(conversation.below().running, None);
    let scene = scene_at(&mut conversation, after(now, 900));
    assert!(
        scene
            .panes
            .iter()
            .flat_map(|pane| &pane.lines)
            .all(|placed| text_of(placed) != "  a")
    );
}
