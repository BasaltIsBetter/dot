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
            color: Paint::ink(Ink::Trace),
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
        assert_eq!(rule.color, Paint::ink(Ink::Faint));
    }
}

#[test]
fn fills_the_gauge_in_white_in_proportion() {
    let layout = layout();
    let strip = Strip {
        gauge: Some(Gauge {
            column: 10,
            fill_per_mille: 500,
            tint: None,
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
    assert_eq!(track.color, Paint::ink(Ink::Trace));
    assert!(close(fill.left, track.left));
    assert!(close(fill.width, 40.0));
    assert_eq!(fill.color, Paint::ink(Ink::Full));
}

#[test]
fn never_fills_a_gauge_past_its_track() {
    let strip = Strip {
        gauge: Some(Gauge {
            column: 0,
            fill_per_mille: 4_000,
            tint: None,
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
            .all(|r| r.color == Paint::ink(Ink::Full))
    );
    assert!(
        rectangles[1..]
            .iter()
            .all(|r| r.left >= layout.strip_x(4).floor())
    );
}

#[test]
fn finds_the_value_clicked_in_the_strip() {
    let layout = layout();
    let strip = Strip {
        targets: vec![(Target::Model, [0, 5]), (Target::Settings, [10, 11])],
        ..Strip::default()
    };
    let middle = layout.strip_middle();

    assert_eq!(
        strip.target_at(&layout, [layout.strip_x(0), middle]),
        Some(Target::Model)
    );
    assert_eq!(
        strip.target_at(&layout, [layout.strip_x(4), middle]),
        Some(Target::Model)
    );
    assert_eq!(strip.target_at(&layout, [layout.strip_x(5), middle]), None);
    assert_eq!(
        strip.target_at(&layout, [layout.strip_x(10) + 1.0, middle]),
        Some(Target::Settings)
    );
    assert_eq!(strip.target_at(&layout, [layout.strip_x(11), middle]), None);
    assert_eq!(
        strip.target_at(&layout, [layout.strip_x(2), layout.strip_top() - 1.0]),
        None
    );
    assert_eq!(
        strip.target_at(&layout, [layout.strip_x(2), layout.strip_top()]),
        Some(Target::Model)
    );
}

fn aimed_strip() -> Strip {
    Strip {
        text: Line::new(
            2,
            vec![
                dot_transcript::Span::new("Opus", Ink::Full),
                dot_transcript::Span::new(" ", Ink::Secondary),
                dot_transcript::Span::new("◇ high", Ink::Secondary).heated(40),
                dot_transcript::Span::new(" ", Ink::Secondary),
                dot_transcript::Span::new("writes", Ink::Live),
                dot_transcript::Span::new(" ", Ink::Secondary),
                dot_transcript::Span::new("◇", Ink::Faint),
            ],
        ),
        targets: vec![
            (Target::Model, [2, 6]),
            (Target::Effort, [7, 13]),
            (Target::Settings, [21, 22]),
        ],
        ..Strip::default()
    }
}

fn inks(strip: &Strip) -> Vec<Ink> {
    strip.text.spans.iter().map(|span| span.ink).collect()
}

#[test]
fn lights_the_effort_and_the_gear_under_the_pointer_like_the_prototype() {
    let layout = layout();
    let on_the_bar = layout.strip_middle();
    let mut effort = aimed_strip();
    let mut gear = aimed_strip();

    effort.light_under(&layout, [layout.strip_x(8), on_the_bar]);
    gear.light_under(&layout, [layout.strip_x(21) + 1.0, on_the_bar]);

    assert_eq!(effort.text.spans[2].ink, Ink::Full);
    assert_eq!(effort.text.spans[2].heat, 40);
    assert_eq!(effort.text.spans[6].ink, Ink::Faint);
    assert_eq!(gear.text.spans[6].ink, Ink::Full);
    assert_eq!(gear.text.spans[2].ink, Ink::Secondary);
}

#[test]
fn lights_nothing_off_a_target_or_above_the_bar() {
    let layout = layout();
    let resting = inks(&aimed_strip());
    let mut between = aimed_strip();
    let mut above = aimed_strip();

    between.light_under(&layout, [layout.strip_x(16), layout.strip_middle()]);
    above.light_under(&layout, [layout.strip_x(8), layout.strip_top() - 1.0]);

    assert_eq!(inks(&between), resting);
    assert_eq!(inks(&above), resting);
}

#[test]
fn never_lights_what_is_live() {
    let layout = layout();
    let mut strip = Strip {
        targets: vec![(Target::Effort, [7, 21])],
        ..aimed_strip()
    };

    strip.light_under(&layout, [layout.strip_x(8), layout.strip_middle()]);

    assert_eq!(strip.text.spans[4].ink, Ink::Live);
    assert_eq!(strip.text.spans[2].ink, Ink::Full);
}
