use std::time::Duration;

use dot_api::{Column, Hue, Model, Text};
use dot_transcript::opacity;
use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::settings::reel::place_of;
use crate::settings::tests::test_settings;

const RETINA: f64 = 2.0;

pub(crate) fn layout() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), RETINA)
}

pub(crate) fn frame(layout: &Layout) -> Frame {
    Frame::of(layout, 900.0, 1700.0)
}

pub(crate) fn look<'a>(settings: &'a Settings, defaults: &'a Settings) -> Look<'a, IdTranslator> {
    Look {
        settings,
        defaults,
        translator: &IdTranslator,
        hover: None,
        changed: None,
        actions: &NO_ACTIONS,
        waiting: Waiting::Nothing,
    }
}

static NO_ACTIONS: Actions = Actions {
    folder: String::new(),
    account: String::new(),
    key: String::new(),
};

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn drawn_with(reel: &Reel, look: &Look<'_, IdTranslator>) -> Drawn {
    let layout = layout();
    entries(reel, look, (&layout, &frame(&layout)), Instant::now())
}

fn choices_of(drawn: &Drawn, setting: Setting) -> Vec<usize> {
    drawn
        .hits
        .iter()
        .filter_map(|(_, hit)| match hit {
            Hit::Choose(chosen, option) if *chosen == setting => Some(*option),
            _ => None,
        })
        .collect()
}

fn texts(panes: &[Pane]) -> Vec<String> {
    panes
        .iter()
        .flat_map(|pane| &pane.lines)
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

fn alpha_of(panes: &[Pane], text: &str) -> u8 {
    panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .find(|placed| placed.line.spans.iter().any(|span| span.text == text))
        .map(|placed| placed.line.alpha)
        .unwrap()
}

#[test]
fn offers_every_option_of_the_setting_in_focus_and_only_the_chosen_one_elsewhere() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    assert_eq!(choices_of(&drawn, Setting::Model), [0, 1, 2, 3]);
    assert_eq!(choices_of(&drawn, Setting::Effort), [2]);
    assert_eq!(choices_of(&drawn, Setting::Column), [2]);
}

#[test]
fn lets_each_setting_be_clicked_into_focus() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    let focused: Vec<usize> = drawn
        .hits
        .iter()
        .filter_map(|(_, hit)| match hit {
            Hit::Focus(place) => Some(*place),
            Hit::Choose(..) => None,
        })
        .collect();
    assert!(focused.starts_with(&[1, 2, 3, 4, 5, 7, 8]), "{focused:?}");
    for section in [0, 6, 16, 22] {
        assert!(!focused.contains(&section), "{focused:?}");
    }
}

#[test]
fn writes_each_label_and_section_title() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    let texts = texts(&drawn.panes);
    for text in [
        Text::SettingsConversation,
        Text::SettingsDisplay,
        Text::SettingModel,
        Text::SettingSessionsColumn,
    ] {
        assert!(texts.iter().any(|shown| shown == text.id()), "{text:?}");
    }
    assert!(
        texts
            .iter()
            .any(|shown| shown.starts_with("Haiku 5.5  Sonnet 5.5"))
    );
}

#[test]
fn draws_a_rule_after_each_section_title() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    let rules = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.ink == RULE_INK)
        .count();
    assert_eq!(rules, 3);
}

#[test]
fn underlines_the_chosen_option_of_the_setting_in_focus() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    let underlines: Vec<_> = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.ink == FOCUSED_INK && mark.height <= HAIRLINE)
        .collect();
    assert_eq!(underlines.len(), 1);
    let opus = drawn
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Choose(Setting::Model, 2))
        .unwrap()
        .0;
    assert!((underlines[0].left - opus[0]).abs() < 0.01);
    assert!((underlines[0].width - opus[2]).abs() < 0.01);
}

#[test]
fn marks_the_settings_changed_from_their_default() {
    let defaults = test_settings();
    let mut settings = test_settings();
    let unchanged = drawn_with(&Reel::new(), &look(&settings, &defaults));
    settings.column = Column::Narrow;
    settings.tuning.model = Model::HAIKU_5_5;

    let changed = drawn_with(&Reel::new(), &look(&settings, &defaults));

    let marked = |drawn: &Drawn| {
        drawn
            .marks
            .iter()
            .filter(|mark| mark.color.ink == MARK_INK)
            .count()
    };
    assert_eq!(marked(&unchanged), 0);
    assert_eq!(marked(&changed), 2);
}

#[test]
fn shows_a_swatch_for_each_colour_and_rings_the_chosen_one_in_focus() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.focus_on(place_of(Setting::Color), now);
    let mut settings = test_settings();
    settings.color = Hue::Blue;

    let drawn = drawn_with(&reel, &look(&settings, &test_settings()));

    let swatches: Vec<&Rectangle> = drawn
        .marks
        .iter()
        .filter(|mark| matches!(mark.color.ink, Ink::Rgb(_)))
        .collect();
    assert_eq!(swatches.len(), 8);
    assert_eq!(swatches[0].color.ink, Ink::Rgb(Hue::Orange.rgb()));
    assert_eq!(swatches[5].color.ink, Ink::Rgb(Hue::Blue.rgb()));
    assert_eq!(swatches[5].color.dim, u8::MAX);
    assert!(swatches[0].color.dim < u8::MAX);
    assert!(swatches[6].color.dim < u8::MAX);
    let ring = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.ink == FOCUSED_INK)
        .count();
    assert_eq!(ring, 4);
    assert_eq!(
        choices_of(&drawn, Setting::Color),
        (0..8).collect::<Vec<_>>()
    );
    assert!(
        texts(&drawn.panes)
            .iter()
            .any(|text| text == Text::HueBlue.id())
    );
}

#[test]
fn shows_only_the_chosen_colour_out_of_focus() {
    let settings = test_settings();

    let drawn = drawn_with(&Reel::new(), &look(&settings, &settings));

    let swatches: Vec<&Rectangle> = drawn
        .marks
        .iter()
        .filter(|mark| matches!(mark.color.ink, Ink::Rgb(_)))
        .collect();
    assert_eq!(swatches.len(), 1);
    assert_eq!(swatches[0].color.ink, Ink::Rgb(Hue::Violet.rgb()));
}

#[test]
fn dims_the_settings_a_search_leaves_out() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.type_query("effort", &IdTranslator, now);
    let settings = test_settings();

    let drawn = drawn_with(&reel, &look(&settings, &settings));

    let found = alpha_of(&drawn.panes, Text::SettingEffort.id());
    let left_out = alpha_of(&drawn.panes, Text::SettingColumn.id());
    assert!(left_out < found / 3, "{left_out} {found}");
}

#[test]
fn draws_nothing_below_the_floor() {
    let layout = layout();
    let settings = test_settings();
    let low = Frame::of(&layout, 900.0, 1000.0);

    let drawn = entries(
        &Reel::new(),
        &look(&settings, &settings),
        (&layout, &low),
        Instant::now(),
    );

    assert!(drawn.hits.iter().all(|(bounds, _)| bounds[1] <= 1000.0));
    assert_eq!(choices_of(&drawn, Setting::SessionsColumn), []);
}

#[test]
fn lights_the_row_under_the_pointer() {
    let settings = test_settings();
    let layout = layout();
    let frame = frame(&layout);
    let resting = drawn_with(&Reel::new(), &look(&settings, &settings));
    let effort = resting
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Focus(2))
        .unwrap()
        .0;
    let hovering = Look {
        hover: Some([effort[0] + 1.0, effort[1] + 1.0]),
        ..look(&settings, &settings)
    };

    let drawn = entries(&Reel::new(), &hovering, (&layout, &frame), Instant::now());

    let ink_of = |drawn: &Drawn| {
        drawn
            .panes
            .iter()
            .flat_map(|pane| &pane.lines)
            .flat_map(|placed| &placed.line.spans)
            .find(|span| span.text == Text::SettingEffort.id())
            .map(|span| span.ink)
    };
    assert_eq!(ink_of(&resting), Some(RESTING_INK));
    assert_eq!(ink_of(&drawn), Some(FOCUSED_INK));
}

#[test]
fn heats_the_value_just_changed_then_cools_it_in_a_second_and_a_fifth() {
    let now = Instant::now();
    let changed = Some((Setting::Effort, now));

    assert_eq!(heat_of(Setting::Effort, changed, now), u8::MAX);
    assert!(heat_of(Setting::Effort, changed, after(now, 600)) > 0);
    assert_eq!(heat_of(Setting::Effort, changed, after(now, 1_200)), 0);
    assert_eq!(heat_of(Setting::Model, changed, now), 0);
    assert_eq!(heat_of(Setting::Model, None, now), 0);
    assert!(is_cooling(changed, after(now, 1_100)));
    assert!(!is_cooling(changed, after(now, 1_300)));
    assert!(!is_cooling(None, now));
}

fn top_of(drawn: &Drawn, wanted: Hit) -> Option<f32> {
    drawn
        .hits
        .iter()
        .find(|(_, hit)| *hit == wanted)
        .map(|(bounds, _)| bounds[1])
}

fn drawn_in(frame: &Frame) -> Drawn {
    let layout = layout();
    let settings = test_settings();
    entries(
        &Reel::new(),
        &look(&settings, &settings),
        (&layout, frame),
        Instant::now(),
    )
}

#[test]
fn lays_the_row_in_focus_on_the_band_and_the_next_ones_under_its_hint() {
    let layout = layout();
    let frame = frame(&layout);
    let row = layout.scaled(ROW);
    let hint = layout.scaled(HINT_SPACE);

    let drawn = drawn_in(&frame);

    assert_eq!(top_of(&drawn, Hit::Focus(1)), Some(frame.band_top));
    assert_eq!(
        top_of(&drawn, Hit::Focus(2)),
        Some((frame.band_top + row + hint).round())
    );
    assert_eq!(
        top_of(&drawn, Hit::Focus(3)),
        Some((frame.band_top + 2.0 * row + hint).round())
    );
    let gap = layout.scaled(draw::SECTION_GAP);
    assert_eq!(
        top_of(&drawn, Hit::Focus(7)),
        Some((frame.band_top + 6.0 * row + gap + hint).round())
    );
}

#[test]
fn keeps_a_row_that_ends_at_the_top_of_the_window_and_drops_one_above() {
    let layout = layout();
    let row = layout.scaled(ROW);
    let at_the_top = Frame {
        band_top: -row,
        ..frame(&layout)
    };
    let above = Frame {
        band_top: -row - 1.0,
        ..frame(&layout)
    };

    assert_eq!(top_of(&drawn_in(&at_the_top), Hit::Focus(1)), Some(-row));
    assert_eq!(top_of(&drawn_in(&above), Hit::Focus(1)), None);
}

#[test]
fn keeps_a_row_that_starts_on_the_floor_and_drops_one_below() {
    let layout = layout();
    let frame = frame(&layout);
    let on_the_floor = Frame {
        floor: frame.band_top,
        ..frame
    };

    let drawn = drawn_in(&on_the_floor);

    assert!(top_of(&drawn, Hit::Focus(1)).is_some());
    assert_eq!(top_of(&drawn, Hit::Focus(2)), None);
}

#[test]
fn fades_a_row_by_how_close_its_middle_is_to_the_floor() {
    let layout = layout();
    let row = layout.scaled(ROW);
    let frame = frame(&layout);
    let near = Frame {
        floor: frame.band_top + 2.0 * row,
        ..frame
    };

    let drawn = drawn_in(&near);

    let expected = opacity(masked(near.band_top + row / 2.0, near.floor));
    assert!(expected < u8::MAX);
    assert_eq!(alpha_of(&drawn.panes, Text::SettingModel.id()), expected);
}

#[test]
fn draws_the_rule_of_a_section_from_after_its_title_to_the_right_edge() {
    let layout = layout();
    let frame = frame(&layout);
    let row = layout.scaled(ROW);

    let drawn = drawn_in(&frame);

    let rule = drawn
        .marks
        .iter()
        .find(|mark| mark.color.ink == RULE_INK)
        .copied()
        .unwrap();
    let title = Text::SettingsConversation.id();
    let start = frame.left
        + layout.advance_of(TextSize::Small) * draw::width_of(title)
        + layout.scaled(RULE_GAP);
    let section_top = frame.band_top - row;
    assert!((rule.left - start).abs() < 0.01);
    assert!((rule.top - (section_top + row / 2.0).round()).abs() < 0.01);
    assert!((rule.left + rule.width - frame.right()).abs() < 0.01);
}

#[test]
fn draws_no_rule_when_the_title_fills_the_row() {
    let layout = layout();
    let title = Text::SettingsConversation.id();
    let exact =
        layout.advance_of(TextSize::Small) * draw::width_of(title) + layout.scaled(RULE_GAP);
    let tight = Frame {
        width: exact,
        ..frame(&layout)
    };

    let drawn = drawn_in(&tight);

    let row = layout.scaled(ROW);
    let conversation_rule = (tight.band_top - row + row / 2.0).round();
    assert!(
        !drawn
            .marks
            .iter()
            .any(|mark| mark.color.ink == RULE_INK && (mark.top - conversation_rule).abs() < 0.01)
    );
}

#[test]
fn marks_a_changed_value_sixteen_points_right_of_the_settings() {
    let layout = layout();
    let frame = frame(&layout);
    let row = layout.scaled(ROW);
    let defaults = test_settings();
    let mut settings = test_settings();
    settings.tuning.model = Model::HAIKU_5_5;

    let drawn = entries(
        &Reel::new(),
        &look(&settings, &defaults),
        (&layout, &frame),
        Instant::now(),
    );

    let mark = drawn
        .marks
        .iter()
        .find(|mark| mark.color.ink == MARK_INK)
        .copied()
        .unwrap();
    assert!((mark.left - (frame.right() + layout.scaled(MARK_FROM_RIGHT))).abs() < 0.01);
    assert!((mark.top - (frame.band_top + row / 2.0).round()).abs() < 0.01);
    assert!((mark.width - layout.scaled(MARK_WIDTH)).abs() < 0.01);
}

#[test]
fn lines_the_options_up_to_the_right_edge_two_spaces_apart() {
    let layout = layout();
    let frame = frame(&layout);
    let row = layout.scaled(ROW);
    let advance = layout.advance_of(TextSize::Regular);

    let drawn = drawn_in(&frame);

    let spots: Vec<[f32; 4]> = (0..4)
        .map(|option| {
            drawn
                .hits
                .iter()
                .find(|(_, hit)| *hit == Hit::Choose(Setting::Model, option))
                .unwrap()
                .0
        })
        .collect();
    for (option, spot) in spots.iter().enumerate() {
        let name = Setting::Model.option_name(option, &IdTranslator);
        assert!((spot[2] - draw::width_of(&name) * advance).abs() < 0.01);
        assert!((spot[1] - frame.band_top).abs() < 0.01);
        assert!((spot[3] - row).abs() < 0.01);
    }
    for pair in spots.windows(2) {
        assert!((pair[1][0] - (pair[0][0] + pair[0][2] + 2.0 * advance)).abs() < 0.01);
    }
    let last = spots[3];
    assert!((last[0] + last[2] - frame.right()).abs() < 0.01);
    let underline = drawn
        .marks
        .iter()
        .find(|mark| mark.color.ink == FOCUSED_INK && mark.height <= HAIRLINE)
        .copied()
        .unwrap();
    let expected_top = frame.band_top + row - layout.scaled(UNDERLINE_FROM_BOTTOM);
    assert!((underline.top - expected_top).abs() < 0.01);
    assert!((underline.left - spots[2][0]).abs() < 0.01);
}

#[test]
fn ends_the_swatches_fourteen_points_before_the_colour_s_name() {
    let layout = layout();
    let frame = frame(&layout);
    let row = layout.scaled(ROW);
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.focus_on(place_of(Setting::Color), now);
    let settings = test_settings();

    let drawn = entries(
        &reel,
        &look(&settings, &settings),
        (&layout, &frame),
        now + Duration::from_secs(1),
    );

    let swatches: Vec<Rectangle> = drawn
        .marks
        .iter()
        .filter(|mark| matches!(mark.color.ink, Ink::Rgb(_)))
        .copied()
        .collect();
    let name = Setting::Color.option_name(Setting::Color.chosen(&settings), &IdTranslator);
    let name_left = frame.right() - draw::width_of(&name) * layout.advance_of(TextSize::Regular);
    let last = swatches[7];
    assert!((last.left + last.width + layout.scaled(SWATCH_TO_NAME) - name_left).abs() < 0.01);
    assert!(
        (swatches[2].left - (swatches[1].left + layout.scaled(SWATCH + SWATCH_GAP))).abs() < 0.01
    );
    let swatch = layout.scaled(SWATCH);
    let row_top = top_of(&drawn, Hit::Focus(place_of(Setting::Color))).unwrap();
    assert!((last.top - (row_top + (row - swatch) / 2.0).round()).abs() < 0.01);
}

fn colour_drawn(hover: Option<[f32; 2]>) -> Drawn {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.focus_on(place_of(Setting::Color), now);
    let settings = test_settings();
    let looked = Look {
        hover,
        ..look(&settings, &settings)
    };
    let layout = layout();
    entries(&reel, &looked, (&layout, &frame(&layout)), now)
}

fn swatches_of(drawn: &Drawn) -> Vec<Rectangle> {
    drawn
        .marks
        .iter()
        .filter(|mark| matches!(mark.color.ink, Ink::Rgb(_)))
        .copied()
        .collect()
}

#[test]
fn draws_every_swatch_whole_and_evenly_spaced() {
    let swatches = swatches_of(&colour_drawn(None));

    let side = layout().scaled(SWATCH);
    let step = side + layout().scaled(SWATCH_GAP);
    for (place, swatch) in swatches.iter().enumerate() {
        assert!((swatch.width - side).abs() < 0.01, "{place}");
        assert!((swatch.height - side).abs() < 0.01, "{place}");
        assert!((swatch.top - swatches[0].top).abs() < 0.01, "{place}");
        assert!(
            (swatch.left - swatches[0].left - to_f32(rows(place)) * step).abs() < 0.01,
            "{place}"
        );
    }
}

#[test]
fn brightens_a_swatch_under_the_pointer_to_four_fifths() {
    let resting = swatches_of(&colour_drawn(None));
    let blue = resting[5];

    let hovered = swatches_of(&colour_drawn(Some([blue.left + 1.0, blue.top + 1.0])));

    assert_eq!(resting[5].color.dim, opacity(SWATCH_RESTING));
    assert_eq!(hovered[5].color.dim, opacity(SWATCH_HOVERED));
    assert_eq!(hovered[4].color.dim, opacity(SWATCH_RESTING));
}

#[test]
fn lets_each_swatch_be_clicked_over_its_width_and_the_gap_after_it() {
    let drawn = colour_drawn(None);
    let swatches = swatches_of(&drawn);
    let layout = layout();

    let blue = drawn
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Choose(Setting::Color, 5))
        .unwrap()
        .0;

    assert!((blue[0] - swatches[5].left).abs() < 0.01);
    assert!((blue[2] - layout.scaled(SWATCH + SWATCH_GAP)).abs() < 0.01);
}

#[test]
fn lights_an_option_under_the_pointer_to_sixty_four_percent() {
    let settings = test_settings();
    let resting = drawn_with(&Reel::new(), &look(&settings, &settings));
    let haiku = resting
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Choose(Setting::Model, 0))
        .unwrap()
        .0;
    let hovering = Look {
        hover: Some([haiku[0] + 1.0, haiku[1] + 1.0]),
        ..look(&settings, &settings)
    };

    let drawn = drawn_with(&Reel::new(), &hovering);

    let ink_of = |drawn: &Drawn, option: usize| {
        let name = Setting::Model.option_name(option, &IdTranslator);
        drawn
            .panes
            .iter()
            .flat_map(|pane| &pane.lines)
            .flat_map(|placed| &placed.line.spans)
            .find(|span| span.text == name)
            .map(|span| span.ink)
    };
    assert_eq!(ink_of(&resting, 0), Some(OTHER_OPTION_INK));
    assert_eq!(ink_of(&drawn, 0), Some(HOVERED_OPTION_INK));
    assert_eq!(ink_of(&drawn, 1), Some(OTHER_OPTION_INK));
}
