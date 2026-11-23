use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::layout::TextSize;
use crate::canvas::mark::Rgba;
use dot_transcript::{Ink, Span};

const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

fn layout() -> Layout {
    Layout::zoomed(WINDOW, 2.0)
}

fn line(indent: u16, text: &str) -> Line {
    Line::new(indent, vec![Span::new(text, Ink::Full)])
}

#[test]
fn an_empty_scene_has_no_text_to_draw() {
    assert_eq!(Scene::default().runs(), Vec::new());
}

#[test]
fn places_grid_lines_on_their_rows_across_the_whole_window() {
    let layout = layout();
    let scene = Scene {
        lines: vec![line(3, "title"), Line::default(), line(0, "hint")],
        layout,
        ..Scene::default()
    };

    let runs = scene.runs();

    assert_eq!(
        runs,
        vec![
            Run {
                line: &scene.lines[0],
                left: layout.x(3),
                top: layout.line_top(0),
                clip: [0.0, 1_800.0],
                size: TextSize::Regular,
            },
            Run {
                line: &scene.lines[2],
                left: layout.x(0),
                top: layout.line_top(2),
                clip: [0.0, 1_800.0],
                size: TextSize::Regular,
            },
        ]
    );
}

#[test]
fn places_pane_lines_where_they_were_put_from_the_pane_left_and_clipped_to_the_pane() {
    let layout = layout();
    let scene = Scene {
        panes: vec![Pane {
            left: layout.x(0) + 24.0,
            size: TextSize::Regular,
            top: 100.0,
            bottom: 300.0,
            lines: vec![Placed {
                top: 80.0,
                line: line(2, "cut at the top"),
                clip: None,
            }],
        }],
        layout,
        ..Scene::default()
    };

    let runs = scene.runs();

    assert_eq!(
        runs,
        vec![Run {
            line: &scene.panes[0].lines[0].line,
            left: layout.x(2) + 24.0,
            top: 80.0,
            clip: [100.0, 300.0],
            size: TextSize::Regular,
        }]
    );
}

#[test]
fn draws_the_strip_text_last_on_its_own_grid() {
    let layout = layout();
    let scene = Scene {
        lines: vec![line(0, "above")],
        strip: Some(Strip {
            text: line(4, "Opus 5.5"),
            ..Strip::default()
        }),
        layout,
        ..Scene::default()
    };

    let runs = scene.runs();

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[1].left, layout.strip_x(4));
    assert_eq!(runs[1].top, layout.strip_text_top());
    assert_eq!(runs[1].size, TextSize::Small);
}

#[test]
fn places_the_columns_of_a_small_pane_with_the_small_advance() {
    let layout = layout();
    let scene = Scene {
        panes: vec![Pane {
            left: 50.0,
            size: TextSize::Small,
            top: 0.0,
            bottom: 300.0,
            lines: vec![Placed {
                top: 10.0,
                line: line(4, "glyph"),
                clip: Some([5.0, 400.0]),
            }],
        }],
        layout,
        ..Scene::default()
    };

    let runs = scene.runs();

    assert_eq!(
        runs[0].left,
        50.0 + 4.0 * layout.advance_of(TextSize::Small)
    );
    assert_eq!(runs[0].size, TextSize::Small);
    assert_eq!(runs[0].clip, [5.0, 300.0]);
}

#[test]
fn fades_and_draws_everything_closer_to_a_point() {
    let white = Rgba::ink(Ink::Full);
    let mut scene = Scene {
        panes: vec![Pane {
            left: 300.0,
            size: TextSize::Regular,
            top: 0.0,
            bottom: 1_000.0,
            lines: vec![Placed {
                top: 500.0,
                line: line(0, "text"),
                clip: None,
            }],
        }],
        marks: vec![Rectangle {
            left: 300.0,
            top: 500.0,
            width: 10.0,
            height: 10.0,
            color: white,
        }],
        layout: layout(),
        ..Scene::default()
    };

    scene.fade_around([100.0, 100.0], 51, 0.5);

    assert_eq!(scene.panes[0].left, 200.0);
    assert_eq!(scene.panes[0].lines[0].top, 300.0);
    assert_eq!(scene.panes[0].lines[0].line.alpha, 51);
    assert_eq!((scene.marks[0].left, scene.marks[0].top), (200.0, 300.0));
    assert_eq!(scene.marks[0].color, white.dimmed(51));
}

#[test]
fn leaves_out_a_line_faded_to_nothing() {
    let scene = Scene {
        lines: vec![line(0, "gone").faded(0.0)],
        layout: layout(),
        ..Scene::default()
    };

    assert_eq!(scene.runs(), Vec::new());
}
