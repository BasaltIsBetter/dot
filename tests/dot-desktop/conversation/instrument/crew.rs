use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;

const RAILED: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);
const RETINA: f64 = 2.0;
const TOOL: [u8; 3] = [0xa9, 0x70, 0xff];
const GRID: Grid = Grid {
    label_right: 400.0,
    value_left: 430.0,
    first_top: 600.0,
    row: 35.0,
};

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn layout() -> Layout {
    Layout::reflowed(RAILED, RETINA)
}

fn reading() -> Reading {
    Reading {
        measures: None,
        heat: 0,
        alpha: OPAQUE,
        values_alpha: OPAQUE,
    }
}

fn look(silhouette: &Silhouette) -> CrewLook<'_> {
    CrewLook {
        silhouette,
        tool: None,
        states: true,
        pulse: OPAQUE,
    }
}

fn wave(count: usize, now: Instant) -> Agents {
    let mut agents = Agents::default();
    for agent in 0..count {
        agents.start(agent.to_string(), "a task".to_owned(), now);
    }
    agents
}

fn drawn(crew: &Crew, reading: &Reading, look: &CrewLook<'_>) -> DrawnCrew {
    let layout = layout();
    crew.drawn(
        (reading, &GRID),
        (&Stage::of(&layout), &layout),
        look,
        &IdTranslator,
    )
}

fn texts(drawn: &DrawnCrew) -> Vec<String> {
    drawn
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .map(|placed| {
            placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        })
        .collect()
}

fn figures(drawn: &DrawnCrew, silhouette: &Silhouette) -> Vec<f32> {
    let mut lefts: Vec<f32> = drawn.marks.iter().map(|mark| mark.left).collect();
    lefts.sort_by(f32::total_cmp);
    let mut starts: Vec<f32> = Vec::new();
    for left in lefts {
        if starts
            .last()
            .is_none_or(|&start| left - start >= silhouette.size()[0])
        {
            starts.push(left);
        }
    }
    starts
}

#[test]
fn shows_nothing_without_an_agent() {
    assert_eq!(Crew::of(&Agents::default(), Instant::now()), None);
}

#[test]
fn shows_the_wave_while_one_works_and_cools_for_a_second_and_a_fifth_after_the_last() {
    let now = Instant::now();
    let mut agents = wave(2, now);
    agents.end("0", after(now, 100));
    let cooling = Crew::of(&agents, after(now, 200)).unwrap();
    let one_cooling = Crew::is_cooling(&agents, after(now, 200));
    agents.end("1", after(now, 300));

    let last = Crew::of(&agents, after(now, 1_499));

    assert_eq!(
        cooling.figures,
        [Figure::Ended(Duration::from_millis(100)), Figure::Working]
    );
    assert_eq!(cooling.working, 1);
    assert!(one_cooling);
    assert!(Crew::is_cooling(&agents, after(now, 1_499)));
    assert!(!Crew::is_cooling(&agents, after(now, 1_500)));
    assert!(last.is_some_and(|crew| crew.working == 0));
    assert_eq!(Crew::of(&agents, after(now, 1_500)), None);
}

#[test]
fn does_not_cool_while_every_agent_works() {
    let now = Instant::now();
    let agents = wave(3, now);
    let crew = Crew::of(&agents, after(now, 5_000)).unwrap();

    assert!(!Crew::is_cooling(&agents, after(now, 5_000)));
    assert!(!Crew::is_cooling(&Agents::default(), now));
    assert_eq!(crew.working, 3);
}

#[test]
fn ends_the_row_with_one_figure_per_agent_centred_in_the_fourth_row() {
    let now = Instant::now();
    let crew = Crew::of(&wave(3, now), now).unwrap();
    let silhouette = Silhouette::at(layout().scale());

    let drawn = drawn(&crew, &reading(), &look(&silhouette));

    let [width, height] = silhouette.size();
    let gap = Stage::of(&layout()).points(3.0);
    let lefts = figures(&drawn, &silhouette);
    let expected: Vec<f32> = [0.0, 1.0, 2.0]
        .iter()
        .map(|at| (GRID.label_right - 3.0 * width - 2.0 * gap + at * (width + gap)).round())
        .collect();
    assert_eq!(lefts, expected);
    let right = drawn
        .marks
        .iter()
        .map(|mark| mark.left + mark.width)
        .fold(f32::MIN, f32::max);
    assert!(
        right <= GRID.label_right + 1.0 && right > GRID.label_right - 2.0,
        "{right}"
    );
    let top = drawn
        .marks
        .iter()
        .map(|mark| mark.top)
        .fold(f32::MAX, f32::min);
    let bottom = drawn
        .marks
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    let row_top = GRID.first_top + 3.0 * GRID.row;
    let middle = row_top + GRID.row / 2.0;
    assert!(
        (f32::midpoint(top, bottom) - middle).abs() <= 1.0,
        "{top} {bottom} {middle}"
    );
    assert!(bottom - top <= height);
}

#[test]
fn labels_the_row_before_its_figures_and_counts_the_agents_at_work() {
    let now = Instant::now();
    let mut agents = wave(3, now);
    agents.end("1", now);
    let crew = Crew::of(&agents, now).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let layout = layout();
    let small = layout.advance_of(TextSize::Small);

    let drawn = drawn(&crew, &reading(), &look(&silhouette));

    assert_eq!(texts(&drawn), ["instrument-agents", "2/3"]);
    let label = &drawn.panes[0];
    let label_right = label.left + width_of("instrument-agents", small);
    let first_figure = figures(&drawn, &silhouette)[0];
    let gap = Stage::of(&layout).points(8.0);
    assert!(
        (first_figure - label_right - gap).abs() <= 1.0,
        "{first_figure} {label_right}"
    );
    assert_eq!(label.lines[0].line.spans[0].ink, Ink::Secondary);
    let value = &drawn.panes[1];
    assert_eq!(value.left, GRID.value_left);
    assert_eq!(value.lines[0].line.indent, 4);
    assert_eq!(value.lines[0].line.spans[0].ink, Ink::Full);
    let row_top = GRID.first_top + 3.0 * GRID.row;
    let text_top = layout.text_top(TextSize::Small, row_top, GRID.row);
    let tops: Vec<f32> = drawn.panes.iter().map(|pane| pane.lines[0].top).collect();
    assert_eq!(tops, [text_top; 2]);
}

#[test]
fn heats_the_count_and_fades_the_row_with_the_instrument() {
    let now = Instant::now();
    let crew = Crew::of(&wave(1, now), now).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let fading = Reading {
        heat: 200,
        alpha: 128,
        values_alpha: 128,
        ..reading()
    };

    let drawn = drawn(&crew, &fading, &look(&silhouette));

    let value = &drawn.panes[1].lines[0].line;
    assert_eq!(value.spans[0].heat, 200);
    assert_eq!(value.alpha, 64);
    assert_eq!(drawn.panes[0].lines[0].line.alpha, 128);
    assert!(drawn.marks.iter().all(|mark| mark.color.dim <= 128));
}

#[test]
fn draws_a_figure_at_work_live_or_in_the_tool_colour_and_pulsing() {
    let silhouette = Silhouette::default();
    let tinted = CrewLook {
        tool: Some(TOOL),
        ..look(&silhouette)
    };
    let now = Instant::now();
    let crew = Crew::of(&wave(1, now), now).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let pulsing = CrewLook {
        pulse: 115,
        ..look(&silhouette)
    };

    let full = drawn(&crew, &reading(), &look(&silhouette));
    let dimmed = drawn(&crew, &reading(), &pulsing);

    assert_eq!(ink_of(Figure::Working, &look(&silhouette)), Ink::Live);
    assert_eq!(ink_of(Figure::Working, &tinted), Ink::Rgb(TOOL));
    let brightest = |drawn: &DrawnCrew| drawn.marks.iter().map(|mark| mark.color.dim).max();
    assert_eq!(brightest(&full), Some(OPAQUE));
    assert_eq!(brightest(&dimmed), Some(115));
}

#[test]
fn cools_a_figure_that_ended_from_green_to_faint() {
    let silhouette = Silhouette::default();
    let states = look(&silhouette);
    let plain = CrewLook {
        states: false,
        ..look(&silhouette)
    };
    let ended = |milliseconds| Figure::Ended(Duration::from_millis(milliseconds));

    let just = ink_of(ended(0), &states);
    let midway = ink_of(ended(300), &states);

    assert_eq!(just, Ink::Rgb(GOOD));
    let Ink::Rgb([red, green, _]) = midway else {
        panic!("{midway:?}");
    };
    assert!(red < GOOD[0] && green < GOOD[1] && green > Ink::Faint.rgb()[1]);
    assert_eq!(ink_of(ended(1_200), &states), Ink::Faint);
    assert_eq!(ink_of(ended(0), &plain), Ink::Faint);
}

#[test]
fn never_pulses_a_figure_that_ended() {
    let now = Instant::now();
    let mut agents = wave(1, now);
    agents.end("0", now);
    let crew = Crew::of(&agents, after(now, 200)).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let pulsing = CrewLook {
        pulse: 115,
        states: false,
        ..look(&silhouette)
    };

    let drawn = drawn(&crew, &reading(), &pulsing);

    assert!(drawn.marks.iter().any(|mark| mark.color.dim == OPAQUE));
}

#[test]
fn shows_one_figure_and_how_many_beyond_five_agents() {
    let now = Instant::now();
    let mut agents = wave(7, now);
    for ended in 0..7 {
        agents.end(&ended.to_string(), after(now, 10 * ended));
    }
    let crew = Crew::of(&agents, after(now, 100)).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let small = layout().advance_of(TextSize::Small);

    let drawn = drawn(&crew, &reading(), &look(&silhouette));

    assert_eq!(texts(&drawn), ["instrument-agents", "×7", "0/7"]);
    assert_eq!(figures(&drawn, &silhouette).len(), 1);
    assert_eq!(crew.shown(), [Figure::Ended(Duration::from_millis(40))]);
    let many = &drawn.panes[1];
    assert!((many.left + 2.0 * small - GRID.label_right).abs() < 0.01);
}

#[test]
fn shows_a_figure_at_work_first_beyond_five_agents() {
    let now = Instant::now();
    let mut agents = wave(6, now);
    agents.end("0", now);

    let crew = Crew::of(&agents, after(now, 100)).unwrap();

    assert_eq!(crew.shown(), [Figure::Working]);
    assert_eq!(Crew::of(&wave(5, now), now).unwrap().shown().len(), 5);
}

#[test]
fn draws_no_figure_under_the_bar() {
    let now = Instant::now();
    let crew = Crew::of(&wave(2, now), now).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let layout = layout();
    let stage = Stage::of(&layout);
    let low = Grid {
        first_top: stage.floor - 3.0 * GRID.row - 10.0,
        ..GRID
    };

    let drawn = crew.drawn(
        (&reading(), &low),
        (&stage, &layout),
        &look(&silhouette),
        &IdTranslator,
    );

    assert_ne!(drawn.marks.len(), 0);
    assert!(
        drawn
            .marks
            .iter()
            .all(|mark| mark.top + mark.height <= stage.floor)
    );
}

#[test]
fn counts_beyond_five_agents_only() {
    let now = Instant::now();

    let five = Crew::of(&wave(5, now), now).unwrap();
    let six = Crew::of(&wave(6, now), now).unwrap();

    assert_eq!(five.many(), None);
    assert_eq!(six.many().as_deref(), Some("×6"));
}

#[test]
fn leaves_a_gap_between_the_figure_and_how_many_beyond_five() {
    let now = Instant::now();
    let crew = Crew::of(&wave(6, now), now).unwrap();
    let silhouette = Silhouette::at(layout().scale());
    let gap = Stage::of(&layout()).points(3.0);

    let drawn = drawn(&crew, &reading(), &look(&silhouette));

    let many = &drawn.panes[1];
    let left = figures(&drawn, &silhouette)[0];
    let expected = (many.left - gap - silhouette.size()[0]).round();
    assert!((left - expected).abs() <= 1.0, "{left} {expected}");
}
