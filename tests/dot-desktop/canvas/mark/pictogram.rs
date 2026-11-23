use winit::dpi::PhysicalSize;

use super::*;
use dot_transcript::{Ink, Line, Span};

const SIZE: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);

fn scene_with(lines: Vec<Line>, size: TextSize) -> Scene {
    let layout = Layout::reflowed(SIZE, 2.0);
    let placed = lines
        .into_iter()
        .zip(0_u16..)
        .map(|(line, index)| Placed {
            top: 100.0 + f32::from(index) * 40.0,
            line,
            clip: None,
        })
        .collect();
    Scene {
        layout,
        panes: vec![Pane {
            left: 50.0,
            size,
            top: 0.0,
            bottom: 1_000.0,
            lines: placed,
        }],
        ..Scene::default()
    }
}

fn texts(scene: &Scene) -> Vec<String> {
    scene.panes[0]
        .lines
        .iter()
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

#[test]
fn draws_a_pictogram_the_font_lacks_in_its_place_and_leaves_a_space() {
    let mut scene = scene_with(
        vec![Line::new(0, vec![Span::new("⧉ explore", Ink::Full)])],
        TextSize::Regular,
    );
    let expected: Vec<Rectangle> = Icon::Tool(Glyph::Agents)
        .drawn(
            [
                50.0,
                (100.0
                    + (scene.layout.line_height_of(TextSize::Regular)
                        - Icon::height() * scene.layout.icon_dot())
                        / 2.0)
                    .round(),
            ],
            scene.layout.icon_dot(),
            Rgba::shaded(Ink::Full, 0, dot_transcript::OPAQUE),
        )
        .collect();

    scene.draw_pictograms();

    assert_eq!(texts(&scene), ["  explore"]);
    assert_eq!(scene.marks, expected);
}

#[test]
fn counts_columns_across_spans_and_from_the_indent() {
    let mut scene = scene_with(
        vec![Line::new(
            2,
            vec![
                Span::new("── tests  ", Ink::Full),
                Span::new("↗ ▤ layout.rs", Ink::Secondary),
            ],
        )],
        TextSize::Small,
    );
    let advance = scene.layout.advance_of(TextSize::Small);

    scene.draw_pictograms();

    let left = scene
        .marks
        .iter()
        .map(|mark| mark.left)
        .fold(f32::MAX, f32::min);
    assert_eq!(left, (50.0 + 14.0 * advance).round());
    assert_eq!(texts(&scene), ["── tests  ↗   layout.rs"]);
    assert!(
        scene
            .marks
            .iter()
            .all(|mark| mark.color == Rgba::shaded(Ink::Secondary, 0, dot_transcript::OPAQUE))
    );
}

#[test]
fn takes_the_heat_and_the_opacity_of_its_line() {
    let mut line = Line::new(0, vec![Span::new("✎", Ink::Full).heated(255)]);
    line.alpha = 100;
    let mut scene = scene_with(vec![line], TextSize::Regular);

    scene.draw_pictograms();

    assert_ne!(scene.marks, []);
    assert!(
        scene
            .marks
            .iter()
            .all(|mark| mark.color == Rgba::shaded(Ink::Full, 255, 100))
    );
}

#[test]
fn leaves_a_line_without_pictograms_as_it_is() {
    let line = Line::new(0, vec![Span::new("⌘ cargo test ✓", Ink::Full)]);
    let mut scene = scene_with(vec![line.clone()], TextSize::Regular);

    scene.draw_pictograms();

    assert_eq!(scene.marks, []);
    assert_eq!(scene.panes[0].lines[0].line, line);
}

#[test]
fn draws_nothing_outside_what_its_line_shows() {
    let mut scene = scene_with(
        vec![Line::new(0, vec![Span::new("⌕ x", Ink::Full)])],
        TextSize::Regular,
    );
    scene.panes[0].lines[0].clip = Some([500.0, 900.0]);

    scene.draw_pictograms();

    assert_eq!(scene.marks, []);
    assert_eq!(texts(&scene), ["  x"]);
}

#[test]
fn knows_each_pictogram_the_font_lacks() {
    let known: Vec<Option<Icon>> = ['▤', '⌕', '✎', '✚', '⧉', '↶', '↷', '⌘', 'a']
        .into_iter()
        .map(glyph_of)
        .collect();

    assert_eq!(
        known,
        [
            Some(Icon::Tool(Glyph::Read)),
            Some(Icon::Tool(Glyph::Search)),
            Some(Icon::Tool(Glyph::Edit)),
            Some(Icon::Tool(Glyph::Create)),
            Some(Icon::Tool(Glyph::Agents)),
            Some(Icon::Back),
            Some(Icon::Forward),
            None,
            None,
        ]
    );
}
