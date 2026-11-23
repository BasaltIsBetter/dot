use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::mark::shape::Rgba;
use dot_transcript::Ink;

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

fn every_icon() -> Vec<Icon> {
    let glyphs = [
        Glyph::Read,
        Glyph::Search,
        Glyph::Edit,
        Glyph::Create,
        Glyph::Command,
        Glyph::Agents,
        Glyph::Web,
        Glyph::Other,
    ];
    std::iter::once(Icon::Model)
        .chain(glyphs.map(Icon::Tool))
        .collect()
}

#[test]
fn draws_every_icon_in_the_same_nine_by_seven_box() {
    for icon in every_icon() {
        assert_eq!(icon.rows().len(), 7, "{icon:?}");
        assert!(icon.rows().iter().all(|row| row.len() == 9), "{icon:?}");
        assert!(dots_of(icon) > 0, "{icon:?}");
    }
}

#[test]
fn covers_exactly_the_dots_of_the_icon() {
    let layout = layout();
    let white = Rgba::ink(Ink::Full);
    for icon in every_icon() {
        let area: f32 = icon
            .in_the_strip(&layout, 0, white)
            .map(|rectangle| rectangle.width * rectangle.height)
            .sum();

        let dot_area = layout.icon_dot() * layout.icon_dot();
        let dots = f32::from(u8::try_from(dots_of(icon)).unwrap());
        assert!((area - dot_area * dots).abs() < 0.01, "{icon:?}");
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
fn centres_the_icon_in_its_two_cells_across_the_middle_of_the_strip() {
    let layout = layout();
    let width = 9.0 * layout.icon_dot();
    let rectangles: Vec<Rectangle> = Icon::Model
        .in_the_strip(&layout, 5, Rgba::ink(Ink::Full))
        .collect();
    let left = rectangles.iter().map(|r| r.left).fold(f32::MAX, f32::min);
    let top = rectangles.iter().map(|r| r.top).fold(f32::MAX, f32::min);
    let bottom = rectangles
        .iter()
        .map(|r| r.top + r.height)
        .fold(f32::MIN, f32::max);

    let cells_middle = layout.strip_x(5) + layout.advance_of(TextSize::Small);
    assert!((left + width / 2.0 - cells_middle).abs() <= 1.0);
    assert!((f32::midpoint(top, bottom) - layout.strip_middle()).abs() <= 1.0);
    assert!(rectangles.iter().all(|r| r.color == Rgba::ink(Ink::Full)));
}

#[test]
fn grows_its_dots_with_the_text() {
    let dot_at = |scale_factor: f64| (1.5 * 0.8 * scale_factor).round();

    assert!((layout().icon_dot() - 2.0).abs() < 0.01);
    assert!((dot_at(RETINA) - 2.0).abs() < 0.01);
    let small = Layout::zoomed(PhysicalSize::new(1440, 900), 1.0);
    assert!((f64::from(small.icon_dot()) - dot_at(1.0)).abs() < 0.01);
    assert!(small.icon_dot() < layout().icon_dot());
}

#[test]
fn draws_each_run_of_dots_at_its_place_one_dot_high() {
    let colour = Rgba::ink(Ink::Full);

    let drawn: Vec<(f32, f32, f32, f32)> = Icon::Tool(Glyph::Read)
        .drawn([10.0, 20.0], 2.0, colour)
        .take(3)
        .map(|rectangle| {
            (
                rectangle.left,
                rectangle.top,
                rectangle.width,
                rectangle.height,
            )
        })
        .collect();

    assert_eq!(
        drawn,
        [
            (10.0, 20.0, 18.0, 2.0),
            (10.0, 22.0, 2.0, 2.0),
            (26.0, 22.0, 2.0, 2.0)
        ]
    );
    assert!((Icon::height() - 7.0).abs() < f32::EPSILON);
}

#[test]
fn centres_a_wide_icon_in_its_two_cells_on_a_dense_screen() {
    let layout = Layout::reflowed(PhysicalSize::new(4_000, 2_400), 3.0);
    let width = 9.0 * layout.icon_dot();

    let left = Icon::Model
        .in_the_strip(&layout, 5, Rgba::ink(Ink::Full))
        .map(|rectangle| rectangle.left)
        .fold(f32::MAX, f32::min);

    let cells = 2.0 * layout.advance_of(TextSize::Small);
    let expected = layout.strip_x(5) + (cells - width) / 2.0;
    assert!((left - expected).abs() <= 0.5, "{left} {expected}");
}

#[test]
fn draws_the_arrows_of_the_editor_as_mirrors_of_each_other() {
    let mirrored = |row: &str| row.chars().rev().collect::<String>();

    assert!(
        Icon::Back
            .rows()
            .iter()
            .zip(Icon::Forward.rows())
            .all(|(back, forward)| mirrored(back) == forward)
    );
    assert_eq!(Icon::Back.rows()[2], "#######..");
}
