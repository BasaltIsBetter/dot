use winit::dpi::PhysicalSize;

use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

fn layout() -> Layout {
    Layout::zoomed(WINDOW, RETINA)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn draws_a_faint_hairline_across_the_top_of_an_empty_strip() {
    let rectangles = Strip::default().rectangles(&layout());

    assert_eq!(
        rectangles,
        [Rectangle {
            left: 0.0,
            top: 1755.0,
            width: 2880.0,
            height: 1.0,
            color: Rgba::ink(Ink::Trace),
        }]
    );
}

#[test]
fn stands_the_rules_of_the_middle_zone_across_the_middle_of_the_strip() {
    let layout = layout();
    let strip = Strip {
        rules: vec![3, 20],
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout);

    assert_eq!(rectangles.len(), 3);
    for (rule, column) in rectangles[1..].iter().zip([3, 20]) {
        assert!(close(rule.left, layout.strip_x(column).round()));
        assert!(close(rule.top, 1766.0));
        assert!(close(rule.width, 1.0));
        assert!(close(rule.height, 22.0));
        assert_eq!(rule.color, Rgba::ink(Ink::Faint));
    }
}

#[test]
fn fills_the_gauge_in_white_in_proportion() {
    let layout = layout();
    let strip = Strip {
        gauge: Some(Gauge {
            column: 10,
            fill_per_mille: 500,
        }),
        ..Strip::default()
    };

    let [_, track, fill] = strip.rectangles(&layout)[..] else {
        panic!("a gauge draws a track and its fill");
    };

    assert!(close(track.left, layout.strip_x(10).round()));
    assert!(close(track.width, 80.0));
    assert!(close(track.height, 8.0));
    assert!(close(track.top, 1773.0));
    assert_eq!(track.color, Rgba::ink(Ink::Trace));
    assert!(close(fill.left, track.left));
    assert!(close(fill.width, 40.0));
    assert_eq!(fill.color, Rgba::ink(Ink::Full));
}

#[test]
fn never_fills_a_gauge_past_its_track() {
    let strip = Strip {
        gauge: Some(Gauge {
            column: 0,
            fill_per_mille: 4_000,
        }),
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout());

    assert!(close(rectangles[2].width, rectangles[1].width));
}

#[test]
fn draws_the_robot_in_white_at_its_column() {
    let layout = layout();
    let strip = Strip {
        icons: vec![(4, Icon::Model)],
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout);

    assert!(rectangles.len() > 1);
    assert!(
        rectangles[1..]
            .iter()
            .all(|r| r.color == Rgba::ink(Ink::Full))
    );
    assert!(
        rectangles[1..]
            .iter()
            .all(|r| r.left >= layout.strip_x(4).floor())
    );
}
