use winit::dpi::PhysicalSize;

use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

fn layout() -> Layout {
    Layout::new(WINDOW, RETINA)
}

fn gauge(fill_per_mille: u16) -> Gauge {
    Gauge {
        column: 10,
        width: 6,
        fill_per_mille,
        ink: Ink::Receding,
    }
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn draws_a_hairline_across_the_top_of_an_empty_strip() {
    let rectangles = Strip::default().rectangles(&layout());

    assert_eq!(
        rectangles,
        [Rectangle {
            left: 0.0,
            top: 1744.0,
            width: 2880.0,
            height: 1.0,
            ink: Ink::Faint,
        }]
    );
}

#[test]
fn stands_each_rule_on_its_column_boundary_along_the_text() {
    let layout = layout();
    let strip = Strip {
        rules: vec![3, 20],
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout);

    assert_eq!(rectangles.len(), 3);
    for (rule, column) in rectangles[1..].iter().zip([3, 20]) {
        assert!(close(rule.left, layout.strip_x(column).round()));
        assert!(close(rule.top, layout.strip_text_top()));
        assert!(close(rule.width, 1.0));
        assert!(close(rule.height, 40.0));
        assert_eq!(rule.ink, Ink::Faint);
    }
}

#[test]
fn fills_the_gauge_track_in_proportion() {
    let layout = layout();
    let strip = Strip {
        gauge: Some(gauge(500)),
        ..Strip::default()
    };

    let [_, track, fill] = strip.rectangles(&layout)[..] else {
        panic!("a gauge draws a track and its fill");
    };

    assert!(close(track.left, layout.strip_x(10).round()));
    assert!(close(track.width, 94.0));
    assert!(close(track.height, 8.0));
    assert_eq!(track.ink, Ink::Ground);
    assert!(close(fill.left, track.left));
    assert!(close(fill.width, 47.0));
    assert_eq!(fill.ink, Ink::Receding);
}

#[test]
fn centres_the_gauge_on_the_text_line() {
    let layout = layout();
    let strip = Strip {
        gauge: Some(gauge(0)),
        ..Strip::default()
    };

    let track = strip.rectangles(&layout)[1];

    let middle = track.top + track.height / 2.0;
    assert!(close(
        middle,
        layout.strip_text_top() + layout.line_height() / 2.0
    ));
}

#[test]
fn never_fills_past_the_track() {
    let strip = Strip {
        gauge: Some(gauge(u16::MAX)),
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout());

    assert!(close(rectangles[2].width, rectangles[1].width));
}

#[test]
fn an_empty_gauge_has_no_fill() {
    let strip = Strip {
        gauge: Some(gauge(0)),
        ..Strip::default()
    };

    assert!(close(strip.rectangles(&layout())[2].width, 0.0));
}

#[test]
fn divides_from_the_top_line_down_to_the_bottom_of_the_window() {
    let layout = layout();
    let strip = Strip {
        dividers: vec![9],
        ..Strip::default()
    };

    let rectangles = strip.rectangles(&layout);

    assert_eq!(rectangles.len(), 2);
    let divider = rectangles[1];
    assert!(close(divider.left, layout.strip_x(9).round()));
    assert!(close(divider.top, layout.strip_top()));
    assert!(close(divider.top + divider.height, 1800.0));
    assert!(close(divider.width, 1.0));
    assert_eq!(divider.ink, Ink::Faint);
}
