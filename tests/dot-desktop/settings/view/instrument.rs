use std::time::Duration;

use dot_api::{Column, Hue, LiveColor, Settings};

use super::*;
use crate::app::tests::IdTranslator;
use crate::settings::reel::Step;
use crate::settings::tests::test_settings;
use crate::settings::view::tests::{frame, layout, look};

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

fn measured(
    reel: &Reel,
    settings: &Settings,
    changed: Option<(Setting, Instant)>,
    now: Instant,
) -> (Vec<Pane>, Vec<Rectangle>) {
    let layout = layout();
    let defaults = test_settings();
    let look = Look {
        changed,
        ..look(settings, &defaults)
    };
    instrument(reel, &look, (&layout, &frame(&layout)), now)
}

#[test]
fn lays_the_band_across_the_whole_window() {
    let layout = layout();
    let frame = frame(&layout);

    let band = band(&layout, &frame);

    assert!((band.left - 0.0).abs() < f32::EPSILON);
    assert!((band.width - layout.width()).abs() < f32::EPSILON);
    assert!((band.top - frame.band_top).abs() < f32::EPSILON);
    assert!((band.height - layout.scaled(ROW)).abs() < f32::EPSILON);
    assert_eq!(band.color, Paint::ink(BAND_INK));
}

#[test]
fn explains_the_setting_in_focus_under_the_band() {
    let layout = layout();
    let frame = frame(&layout);

    let hint = hint(Setting::Caret, &IdTranslator, (&layout, &frame));

    assert_eq!(
        texts(std::slice::from_ref(&hint)),
        [Text::SettingCaretHint.id()]
    );
    assert!(hint.lines[0].top > frame.band_top + layout.scaled(ROW));
    assert!((hint.left - frame.left).abs() < f32::EPSILON);
}

#[test]
fn tells_the_default_and_the_place_of_the_setting_in_focus() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.step(Step::Down, &IdTranslator, now);

    let (panes, _) = measured(&reel, &test_settings(), None, now);

    let texts = texts(&panes);
    assert_eq!(
        texts[0],
        format!("{}  {}", Text::SettingsDefault.id(), Text::EffortHigh.id())
    );
    assert_eq!(texts[1], format!("{}  2 / 17", Text::SettingsRank.id()));
}

#[test]
fn ends_left_of_the_settings() {
    let now = Instant::now();
    let layout = layout();
    let frame = frame(&layout);

    let (panes, blocks) = measured(&Reel::new(), &test_settings(), None, now);

    let right = frame.left - layout.scaled(FROM_THE_COLUMN);
    assert!(panes.iter().all(|pane| pane.left < right));
    let last = blocks.last().unwrap();
    assert!((last.left + last.width - right).abs() < 0.01);
}

#[test]
fn measures_a_choice_with_one_block_per_option() {
    let now = Instant::now();
    let mut settings = test_settings();
    settings.tuning.model = dot_api::Model::HAIKU_5_5;

    let (_, blocks) = measured(&Reel::new(), &settings, None, now);

    let inks: Vec<Ink> = blocks.iter().map(|block| block.color.ink).collect();
    assert_eq!(inks, [ON_INK, OFF_INK, DEFAULT_INK, OFF_INK]);
}

#[test]
fn flashes_the_chosen_block_just_after_a_change() {
    let now = Instant::now();

    let (_, flashing) = measured(
        &Reel::new(),
        &test_settings(),
        Some((Setting::Model, now)),
        now,
    );
    let (_, cooled) = measured(
        &Reel::new(),
        &test_settings(),
        Some((Setting::Model, now)),
        now + Duration::from_millis(450),
    );
    let (_, elsewhere) = measured(
        &Reel::new(),
        &test_settings(),
        Some((Setting::Effort, now)),
        now,
    );

    assert_eq!(flashing[2].color.heat, u8::MAX);
    assert_eq!(cooled[2].color.heat, 0);
    assert_eq!(elsewhere[2].color.heat, 0);
}

#[test]
fn measures_the_live_colour_with_its_swatches() {
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.focus_on(crate::settings::reel::place_of(Setting::LiveColor), now);
    let mut settings = test_settings();
    settings.live_color = LiveColor::Fixed(Hue::Pink);

    let (_, blocks) = measured(&reel, &settings, None, now);

    assert_eq!(blocks.len(), 9);
    assert_eq!(blocks[0].color.ink, Ink::Rgb(Hue::Cyan.rgb()));
    assert_eq!(blocks[8].color.ink, Ink::Rgb(Hue::Pink.rgb()));
    assert_eq!(blocks[8].color.dim, u8::MAX);
    assert!(blocks[0].color.dim < u8::MAX);
}

#[test]
fn counts_three_blocks_for_the_column() {
    let now = Instant::now();
    let mut reel = Reel::new();
    for _ in 0..10 {
        reel.step(Step::Down, &IdTranslator, now);
    }
    let settings = Settings {
        column: Column::Narrow,
        ..test_settings()
    };

    let (_, blocks) = measured(&reel, &settings, None, now);

    let inks: Vec<Ink> = blocks.iter().map(|block| block.color.ink).collect();
    assert_eq!(inks, [ON_INK, OFF_INK, DEFAULT_INK]);
}
