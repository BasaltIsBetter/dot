use winit::dpi::PhysicalSize;

use super::*;

const RETINA: f64 = 2.0;

fn layout() -> Layout {
    Layout::zoomed(PhysicalSize::new(2880, 1800), RETINA)
}

fn dots_of(icon: Icon) -> usize {
    icon.rows()
        .iter()
        .map(|row| row.bytes().filter(|&byte| byte == DOT).count())
        .sum()
}

#[test]
fn draws_every_icon_in_the_same_nine_by_seven_box() {
    for icon in [
        Icon::Model,
        Icon::Branch,
        Icon::Lines,
        Icon::Battery,
        Icon::Dollar,
        Icon::Clock,
        Icon::Effort(Effort::Low),
        Icon::Effort(Effort::Max),
    ] {
        assert!(icon.rows().iter().all(|row| row.len() == 9), "{icon:?}");
    }
}

#[test]
fn covers_exactly_the_dots_of_the_icon() {
    let layout = layout();
    for icon in [
        Icon::Model,
        Icon::Branch,
        Icon::Lines,
        Icon::Battery,
        Icon::Dollar,
        Icon::Clock,
    ] {
        let area: f32 = icon
            .rectangles(&layout, 0)
            .iter()
            .map(|rectangle| rectangle.width * rectangle.height)
            .sum();

        let dot_area = layout.icon_dot() * layout.icon_dot();
        assert!((area - dot_area * f32::from(u8::try_from(dots_of(icon)).unwrap())).abs() < 0.01);
    }
}

#[test]
fn merges_a_run_of_dots_into_one_rectangle() {
    assert_eq!(runs("#########").collect::<Vec<_>>(), [(0, 9)]);
    assert_eq!(runs("##.....##").collect::<Vec<_>>(), [(0, 2), (7, 2)]);
    assert_eq!(runs(".#.#.....").collect::<Vec<_>>(), [(1, 1), (3, 1)]);
    assert_eq!(runs(".........").count(), 0);
}

#[test]
fn centres_the_icon_in_its_cell_on_the_text_line() {
    let layout = layout();
    let width = 9.0 * layout.icon_dot();
    let height = 7.0 * layout.icon_dot();
    let rectangles = Icon::Battery.rectangles(&layout, 5);
    let left = rectangles.iter().map(|r| r.left).fold(f32::MAX, f32::min);
    let top = rectangles.iter().map(|r| r.top).fold(f32::MAX, f32::min);
    let top = top - layout.icon_dot();

    let cell_middle = layout.strip_x(5) + layout.advance();
    let line_middle = layout.strip_text_top() + layout.line_height() / 2.0;
    assert!((left + width / 2.0 - cell_middle).abs() <= 1.0);
    assert!((top + height / 2.0 - line_middle).abs() <= 1.0);
    assert!(rectangles.iter().all(|r| r.ink == Ink::Receding));
}

#[test]
fn grows_its_dots_with_the_text() {
    let dot_at = |scale_factor: f64| (1.5 * 0.9 * scale_factor).round();

    assert!((layout().icon_dot() - 3.0).abs() < 0.01);
    assert!((dot_at(RETINA) - 3.0).abs() < 0.01);
    let small = Layout::zoomed(PhysicalSize::new(1440, 900), 1.0);
    assert!((f64::from(small.icon_dot()) - dot_at(1.0)).abs() < 0.01);
    assert!(small.icon_dot() < layout().icon_dot());
}

#[test]
fn closes_the_empty_battery_before_its_terminal() {
    let rows = Icon::Battery.rows();

    assert_eq!(rows[1], "########.");
    assert_eq!(rows[5], "########.");
    assert!(rows[2..5].iter().all(|row| *row == "#......##"));
}

#[test]
fn keeps_every_stroke_of_the_dollar_in_the_same_columns() {
    for row in Icon::Dollar.rows() {
        let filled: Vec<usize> = row.match_indices('#').map(|(column, _)| column).collect();
        assert!(
            filled.iter().all(|&column| (2..=6).contains(&column)),
            "{row}"
        );
    }
}

#[test]
fn fills_one_more_effort_bar_at_each_level() {
    let levels = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::ExtraHigh,
        Effort::Max,
    ];

    let dots: Vec<usize> = levels.map(|effort| dots_of(Icon::Effort(effort))).to_vec();

    assert_eq!(dots, [7, 10, 14, 19, 25]);
    for effort in levels {
        let bottom = Icon::Effort(effort).rows()[6];
        assert_eq!(bottom, "#.#.#.#.#", "{effort:?}");
    }
}
