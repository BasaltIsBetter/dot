use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::tests::{layout, stage};
use dot_transcript::OPAQUE;

const MARKER: [f32; 4] = [1_000.0, 900.0, 1_400.0, 935.0];

fn placed() -> Placement {
    let [left, top, right, bottom] = MARKER;
    Placement {
        arrow: BELOW,
        text: String::new(),
        left,
        top,
        figure_left: left,
        text_left: left,
        right,
        bottom,
    }
}

fn pane_at(left: f32, tops: &[f32]) -> Pane {
    Pane {
        left,
        size: TextSize::Regular,
        top: 0.0,
        bottom: 2_000.0,
        lines: tops
            .iter()
            .map(|&top| Placed {
                top,
                line: Line::new(0, vec![Span::new("x".repeat(40), Ink::Full)]),
                clip: None,
            })
            .collect(),
    }
}

fn mark_at(left: f32, top: f32) -> Rectangle {
    Rectangle {
        left,
        top,
        width: 10.0,
        height: 10.0,
        color: Paint::ink(Ink::Full),
    }
}

fn below() -> Sighting {
    Sighting {
        side: Side::Below,
        row: 40,
        count: 2,
        names: "Explore, tests".to_owned(),
    }
}

#[test]
fn fades_the_lines_that_pass_behind_it() {
    let height = layout().line_height_of(TextSize::Regular);
    let mut panes = [pane_at(900.0, &[880.0, 920.0, 900.0 - height, 935.0])];
    let mut marks: [Rectangle; 0] = [];

    clear_behind(
        &placed(),
        (&mut panes, &mut marks),
        (&stage(), &layout()),
        0.25,
    );

    let alphas: Vec<u8> = panes[0]
        .lines
        .iter()
        .map(|placed| placed.line.alpha)
        .collect();
    assert_eq!(alphas, [64, 64, OPAQUE, OPAQUE]);
}

#[test]
fn leaves_the_lines_beside_it() {
    let advance = layout().advance_of(TextSize::Regular);
    let margin = stage().points(GAP);
    let ends_before = MARKER[0] - margin - 40.0 * advance - 1.0;
    let mut panes = [
        pane_at(ends_before, &[910.0]),
        pane_at(MARKER[2] + margin + 1.0, &[910.0]),
        pane_at(MARKER[2] + margin - 1.0, &[910.0]),
    ];
    let mut marks: [Rectangle; 0] = [];

    clear_behind(
        &placed(),
        (&mut panes, &mut marks),
        (&stage(), &layout()),
        0.0,
    );

    let alphas: Vec<u8> = panes.iter().map(|pane| pane.lines[0].line.alpha).collect();
    assert_eq!(alphas, [OPAQUE, OPAQUE, 0]);
}

#[test]
fn dims_the_marks_behind_it() {
    let mut panes: [Pane; 0] = [];
    let mut marks = [
        mark_at(1_100.0, 905.0),
        mark_at(1_100.0, 950.0),
        mark_at(1_100.0, 895.0),
        mark_at(800.0, 905.0),
    ];

    clear_behind(
        &placed(),
        (&mut panes, &mut marks),
        (&stage(), &layout()),
        0.5,
    );

    let dims: Vec<u8> = marks.iter().map(|mark| mark.color.dim).collect();
    assert_eq!(dims, [128, OPAQUE, 128, OPAQUE]);
}

#[test]
fn holds_a_point_inside_its_row_only() {
    let placed = placed();

    assert!(placed.holds([1_000.0, 900.0]));
    assert!(placed.holds([1_399.0, 934.0]));
    assert!(!placed.holds([999.0, 910.0]));
    assert!(!placed.holds([1_400.0, 910.0]));
    assert!(!placed.holds([1_200.0, 899.0]));
    assert!(!placed.holds([1_200.0, 935.0]));
}

#[test]
fn sits_above_the_bottom_or_under_the_top_centred_on_the_column() {
    let stage = stage();
    let layout = layout();
    let figure = Silhouette::size_at(layout.scale());
    let above = Sighting {
        side: Side::Above,
        ..below()
    };

    let down = placement(&below(), (&stage, &layout), figure, &IdTranslator);
    let up = placement(&above, (&stage, &layout), figure, &IdTranslator);

    assert_eq!(down.arrow, BELOW);
    assert_eq!(up.arrow, ABOVE);
    assert_eq!(
        down.top,
        (stage.height - stage.points(64.0) - stage.row).round()
    );
    assert_eq!(up.top, stage.points(16.0));
    assert!((down.bottom - down.top - stage.row).abs() < 0.01);
    assert_eq!(down.text, "agents-out-of-sight · Explore, tests");
    let middle = f32::midpoint(stage.column[0], stage.column[1]);
    assert!((f32::midpoint(down.left, down.right) - middle).abs() <= 1.0);
    let advance = layout.advance_of(TextSize::Small);
    let gap = stage.points(GAP);
    assert!((down.figure_left - (down.left + advance + gap)).abs() < 0.01);
    assert!((down.text_left - (down.figure_left + figure[0] + gap)).abs() < 0.01);
    assert!((down.right - (down.text_left + 36.0 * advance)).abs() < 0.01);
}

#[test]
fn hides_nothing_that_only_touches_its_edges() {
    let margin = stage().points(GAP);
    let [left, top, right, bottom] = MARKER;
    let mut panes: [Pane; 0] = [];
    let mut marks = [
        Rectangle {
            left: right + margin,
            ..mark_at(0.0, 905.0)
        },
        Rectangle {
            left: left - margin - 10.0,
            ..mark_at(0.0, 905.0)
        },
        mark_at(1_100.0, bottom),
        mark_at(1_100.0, top - 10.0),
        Rectangle {
            left: right + margin - 1.0,
            ..mark_at(0.0, 905.0)
        },
        Rectangle {
            left: left - margin - 9.0,
            ..mark_at(0.0, 905.0)
        },
        mark_at(1_100.0, bottom - 1.0),
        mark_at(1_100.0, top - 9.0),
    ];

    clear_behind(
        &placed(),
        (&mut panes, &mut marks),
        (&stage(), &layout()),
        0.0,
    );

    let dims: Vec<u8> = marks.iter().map(|mark| mark.color.dim).collect();
    assert_eq!(dims, [OPAQUE, OPAQUE, OPAQUE, OPAQUE, 0, 0, 0, 0]);
}

#[test]
fn measures_a_line_from_its_indent_to_its_last_column() {
    let advance = layout().advance_of(TextSize::Regular);
    let margin = stage().points(GAP);
    let line_at = |left: f32, indent: u16, columns: usize| Pane {
        lines: vec![Placed {
            top: 910.0,
            line: Line::new(indent, vec![Span::new("x".repeat(columns), Ink::Full)]),
            clip: None,
        }],
        ..pane_at(left, &[])
    };
    let indent = u16::try_from(index(f64::from(1_100.0 / advance))).unwrap();
    let mut panes = [
        line_at(0.0, indent, 1),
        line_at(0.0, 0, 1),
        line_at(MARKER[0] - margin - 4.0 * advance - 0.5, 0, 4),
        line_at(MARKER[0] - margin - 4.0 * advance - 0.5, 0, 5),
    ];
    let mut marks: [Rectangle; 0] = [];

    clear_behind(
        &placed(),
        (&mut panes, &mut marks),
        (&stage(), &layout()),
        0.0,
    );

    let alphas: Vec<u8> = panes.iter().map(|pane| pane.lines[0].line.alpha).collect();
    assert_eq!(alphas, [0, OPAQUE, OPAQUE, 0]);
}
