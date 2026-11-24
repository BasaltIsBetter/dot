use dot_transcript::Placed;

use super::*;

const ORANGE: [u8; 3] = [0xff, 0x5a, 0x1f];
const STOPPED: &str = "arrêté";

fn everything() -> Tints<'static> {
    Tints {
        tool: Some(ORANGE),
        states: true,
        stopped: STOPPED,
    }
}

fn pane_of(lines: Vec<Line>) -> Pane {
    Pane {
        left: 0.0,
        size: crate::canvas::TextSize::Regular,
        top: 0.0,
        bottom: 100.0,
        lines: lines
            .into_iter()
            .map(|line| Placed {
                top: 0.0,
                line,
                clip: None,
            })
            .collect(),
    }
}

fn tinted(text: &str, ink: Ink, tints: &Tints<'_>) -> Vec<Span> {
    let mut panes = [pane_of(vec![Line::new(0, vec![Span::new(text, ink)])])];
    tint(&mut panes, tints);
    panes[0].lines[0].line.spans.clone()
}

fn pieces(spans: &[Span]) -> Vec<(&str, Ink)> {
    spans
        .iter()
        .map(|span| (span.text.as_str(), span.ink))
        .collect()
}

#[test]
fn tints_the_glyph_that_opens_a_line_in_the_tool_colour() {
    let spans = tinted("  ▤ layout.rs  212 l.", Ink::Full, &everything());

    assert_eq!(
        pieces(&spans),
        [
            ("  ", Ink::Full),
            ("▤", Ink::Rgb(ORANGE)),
            (" layout.rs  212 l.", Ink::Full),
        ]
    );
}

#[test]
fn leaves_a_glyph_inside_a_line_alone() {
    let spans = tinted("Claude lit ▤ ce fichier", Ink::Full, &everything());

    assert_eq!(pieces(&spans), [("Claude lit ▤ ce fichier", Ink::Full)]);
}

#[test]
fn colours_passed_failed_and_stopped_with_their_counts() {
    let spans = tinted(
        "⌘ cargo test  428 ✓ · 1 ✗ · arrêté",
        Ink::Full,
        &Tints {
            tool: None,
            ..everything()
        },
    );

    assert_eq!(
        pieces(&spans),
        [
            ("⌘ cargo test  ", Ink::Full),
            ("428 ✓", Ink::Rgb(GOOD)),
            (" · ", Ink::Full),
            ("1 ✗", Ink::Rgb(BAD)),
            (" · ", Ink::Full),
            ("arrêté", Ink::Rgb(WARN)),
        ]
    );
}

#[test]
fn colours_a_mark_alone_when_no_count_comes_before_it() {
    let spans = tinted("fini ✓ x✗", Ink::Full, &everything());

    assert_eq!(
        pieces(&spans),
        [
            ("fini ", Ink::Full),
            ("✓", Ink::Rgb(GOOD)),
            (" x", Ink::Full),
            ("✗", Ink::Rgb(BAD)),
        ]
    );
}

#[test]
fn tints_quiet_text_in_the_full_colour_like_the_prototype() {
    for ink in [Ink::Secondary, Ink::Receding, Ink::Faint] {
        let spans = tinted("⌘ cargo test  12 ✓", ink, &everything());

        assert_eq!(
            pieces(&spans),
            [
                ("⌘", Ink::Rgb(ORANGE)),
                (" cargo test  ", ink),
                ("12 ✓", Ink::Rgb(GOOD)),
            ]
        );
    }
}

#[test]
fn leaves_dimming_to_the_line_alpha() {
    let mut panes = [pane_of(vec![
        Line::new(0, vec![Span::new("3 ✓", Ink::Secondary)]).faded(0.64),
    ])];

    tint(&mut panes, &everything());

    let line = &panes[0].lines[0].line;
    assert_eq!(line.spans[0].ink, Ink::Rgb(GOOD));
    assert!(line.alpha < u8::MAX);
}

#[test]
fn never_tints_what_is_live() {
    let spans = tinted("⌘ ls  ✓", Ink::Live, &everything());

    assert_eq!(pieces(&spans), [("⌘ ls  ✓", Ink::Live)]);
}

#[test]
fn keeps_the_heat_of_what_it_tints() {
    let mut panes = [pane_of(vec![Line::new(
        0,
        vec![Span::new("3 ✓", Ink::Full).heated(200)],
    )])];

    tint(&mut panes, &everything());

    assert_eq!(panes[0].lines[0].line.spans[0].heat, 200);
}

#[test]
fn tints_the_rule_and_the_glyph_of_an_agent_s_lane() {
    let spans = tinted("── explore  ↗ ▤ layout.rs", Ink::Full, &everything());

    assert_eq!(
        pieces(&spans),
        [
            ("──", Ink::Rgb(ORANGE)),
            (" explore  ↗ ", Ink::Full),
            ("▤", Ink::Rgb(ORANGE)),
            (" layout.rs", Ink::Full),
        ]
    );
}

#[test]
fn leaves_the_marks_of_a_code_block_alone() {
    let spans = tinted("── 12 │ let a = 1;", Ink::Full, &everything());

    assert_eq!(pieces(&spans), [("── 12 │ let a = 1;", Ink::Full)]);
}

#[test]
fn tints_nothing_when_every_colour_is_off() {
    let off = Tints {
        tool: None,
        states: false,
        stopped: STOPPED,
    };

    let spans = tinted("▤ a  3 ✓ arrêté", Ink::Full, &off);

    assert_eq!(pieces(&spans), [("▤ a  3 ✓ arrêté", Ink::Full)]);
}

#[test]
fn tints_no_tool_when_tool_colours_are_off() {
    let states_only = Tints {
        tool: None,
        ..everything()
    };

    let spans = tinted("▤ a", Ink::Full, &states_only);

    assert_eq!(pieces(&spans), [("▤ a", Ink::Full)]);
}

#[test]
fn tints_no_state_when_state_colours_are_off() {
    let tools_only = Tints {
        states: false,
        ..everything()
    };

    let spans = tinted("x 3 ✓", Ink::Full, &tools_only);

    assert_eq!(pieces(&spans), [("x 3 ✓", Ink::Full)]);
}

#[test]
fn finds_marks_across_spans() {
    let mut panes = [pane_of(vec![Line::new(
        0,
        vec![Span::new("12", Ink::Full), Span::new(" ✓", Ink::Secondary)],
    )])];

    tint(&mut panes, &everything());

    assert_eq!(
        pieces(&panes[0].lines[0].line.spans),
        [("12", Ink::Rgb(GOOD)), (" ✓", Ink::Rgb(GOOD))]
    );
}

#[test]
fn an_empty_stopped_word_marks_nothing() {
    let spans = tinted(
        "rien ici",
        Ink::Full,
        &Tints {
            stopped: "",
            ..everything()
        },
    );

    assert_eq!(pieces(&spans), [("rien ici", Ink::Full)]);
}

#[test]
fn counts_only_a_number_right_before_the_mark() {
    assert_eq!(counted_from(&['a', '1', ' ', '✓'], 3), 1);
    assert_eq!(counted_from(&['1', '2', ' ', '✓'], 3), 0);
    assert_eq!(counted_from(&['a', ' ', '✓'], 2), 2);
    assert_eq!(counted_from(&['1', '✓'], 1), 1);
    assert_eq!(counted_from(&['✓'], 0), 0);
}

#[test]
fn only_the_greys_take_a_tint() {
    assert!(!is_grey(Ink::Live));
    assert!(!is_grey(Ink::Rgb([1, 2, 3])));
    for ink in [
        Ink::Full,
        Ink::Secondary,
        Ink::Receding,
        Ink::Faint,
        Ink::Ground,
        Ink::Trace,
    ] {
        assert!(is_grey(ink));
    }
}
