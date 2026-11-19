use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::line::{Ink, Span};

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
            },
            Run {
                line: &scene.lines[2],
                left: layout.x(0),
                top: layout.line_top(2),
                clip: [0.0, 1_800.0],
            },
        ]
    );
}

#[test]
fn places_pane_lines_where_they_were_put_shifted_and_clipped_to_the_pane() {
    let layout = layout();
    let scene = Scene {
        panes: vec![Pane {
            top: 100.0,
            bottom: 300.0,
            shift: 24.0,
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
}
