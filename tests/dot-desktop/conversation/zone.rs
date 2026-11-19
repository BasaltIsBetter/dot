use winit::dpi::PhysicalSize;

use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
const SCALE: f32 = 2.0 * 0.9;
const LINE: f32 = 20.0 * SCALE;
const BAND_ROW: f32 = 40.0;

fn points(value: f32) -> f32 {
    (value * SCALE).round()
}

fn zones(draft_rows: usize) -> (Zones, Layout) {
    let layout = Layout::reflowed(WINDOW, RETINA);
    (Zones::new(&layout, draft_rows), layout)
}

#[test]
fn sits_the_prompter_just_above_the_status_bar() {
    let (zones, layout) = zones(1);

    assert_eq!(zones.prompter_rows, 1);
    assert!((zones.prompter.bottom - (layout.strip_top() - points(32.0))).abs() < f32::EPSILON);
    assert!((zones.prompter.top - (zones.prompter.bottom - LINE)).abs() < f32::EPSILON);
}

#[test]
fn leaves_the_same_room_above_and_below_the_transcript() {
    let (zones, _) = zones(1);

    assert!((zones.transcript.bottom - (zones.prompter.top - points(64.0))).abs() < f32::EPSILON);
    assert!((zones.transcript.top - points(64.0)).abs() < f32::EPSILON);
}

#[test]
fn tightens_every_gap_in_a_compact_window() {
    let layout = Layout::reflowed(PhysicalSize::new(1100, 1800), RETINA);

    let zones = Zones::new(&layout, 1);

    assert_eq!(Density::of(&layout), Density::Compact);
    assert!((zones.prompter.bottom - (layout.strip_top() - points(16.0))).abs() < f32::EPSILON);
    assert!((zones.transcript.bottom - (zones.prompter.top - points(32.0))).abs() < f32::EPSILON);
    assert!((zones.transcript.top - points(32.0)).abs() < f32::EPSILON);
}

#[test]
fn is_regular_in_a_window_that_holds_the_whole_column_and_a_grown_prompter() {
    assert_eq!(
        Density::of(&Layout::reflowed(WINDOW, RETINA)),
        Density::Regular
    );
    assert_eq!(
        Density::of(&Layout::reflowed(PhysicalSize::new(718, 458), 1.0)),
        Density::Regular
    );
}

#[test]
fn is_compact_in_a_window_narrower_than_the_column() {
    let narrow = Layout::reflowed(PhysicalSize::new(717, 900), 1.0);

    assert!(narrow.columns() < Layout::WIDEST_COLUMN);
    assert_eq!(Density::of(&narrow), Density::Compact);
}

#[test]
fn is_compact_in_a_window_too_short_for_a_grown_prompter_and_as_many_rows_above() {
    let short = Layout::reflowed(PhysicalSize::new(1440, 457), 1.0);

    assert_eq!(short.columns(), Layout::WIDEST_COLUMN);
    assert_eq!(Density::of(&short), Density::Compact);
}

#[test]
fn wraps_text_two_columns_short_of_the_column() {
    let narrow = Layout::reflowed(PhysicalSize::new(1100, 1800), RETINA);

    assert_eq!(TextWidth::default().columns(), 94);
    assert_eq!(
        TextWidth::of(&Layout::reflowed(WINDOW, RETINA)),
        TextWidth::default()
    );
    assert_eq!(
        TextWidth::of(&narrow).columns(),
        usize::from(narrow.columns()) - 2
    );
}

#[test]
fn grows_the_prompter_with_its_rows_up_to_eight() {
    let (three, _) = zones(3);
    let (twenty, _) = zones(20);

    assert_eq!(three.prompter_rows, 3);
    assert!((three.prompter.bottom - three.prompter.top - 3.0 * LINE).abs() < f32::EPSILON);
    assert_eq!(twenty.prompter_rows, 8);
    assert!(twenty.transcript.bottom < three.transcript.bottom);
}

#[test]
fn keeps_the_prompter_to_half_of_a_short_window() {
    let layout = Layout::reflowed(PhysicalSize::new(960, 300), 1.0);

    let zones = Zones::new(&layout, 20);

    let between = zones.prompter.bottom - zones.transcript.top;
    let prompter = zones.prompter.bottom - zones.prompter.top;
    assert!(zones.prompter_rows < 8);
    assert!(prompter <= between / 2.0, "{prompter} of {between}");
}

#[test]
fn shows_at_least_one_prompter_row_in_the_smallest_window() {
    let layout = Layout::reflowed(PhysicalSize::new(480, 40), 1.0);

    let zones = Zones::new(&layout, 5);

    assert_eq!(zones.prompter_rows, 1);
    assert!(zones.transcript.rows(layout.line_height()).abs() < f64::EPSILON);
}

#[test]
fn tells_which_area_is_under_the_pointer() {
    let (zones, layout) = zones(1);

    assert_eq!(zones.area_at(0.0), Some(Area::Transcript));
    assert_eq!(
        zones.area_at(zones.prompter.top - 1.0),
        Some(Area::Transcript)
    );
    assert_eq!(zones.area_at(zones.prompter.top), Some(Area::Prompter));
    assert_eq!(
        zones.area_at(layout.strip_top() - 1.0),
        Some(Area::Prompter)
    );
    assert_eq!(zones.area_at(layout.strip_top()), None);
}

#[test]
fn counts_the_rows_a_band_holds() {
    let band = Band {
        top: 100.0,
        bottom: 300.0,
    };

    assert!((band.rows(BAND_ROW) - 5.0).abs() < f64::EPSILON);
    assert!(band.rows(0.0).abs() < f64::EPSILON);
    assert!(
        Band {
            top: 300.0,
            bottom: 100.0
        }
        .rows(BAND_ROW)
        .abs()
            < f64::EPSILON
    );
    assert!((band.row_top(1.5, BAND_ROW) - 160.0).abs() < f32::EPSILON);
}

#[test]
fn centres_a_field_that_widens_by_sixteen_columns() {
    let layout = Layout::reflowed(WINDOW, RETINA);

    assert!((centring(&layout, Some(1)) - 40.0).abs() < f32::EPSILON);
    assert!((centring(&layout, Some(16)) - 40.0).abs() < f32::EPSILON);
    assert!((centring(&layout, Some(17)) - 32.0).abs() < f32::EPSILON);
    assert!((centring(&layout, Some(81)) - 0.0).abs() < f32::EPSILON);
    assert!(centring(&layout, Some(96)).abs() < f32::EPSILON);
    assert!(centring(&layout, Some(200)).abs() < f32::EPSILON);
    assert!(centring(&layout, None).abs() < f32::EPSILON);
}

#[test]
fn caps_the_last_step_at_a_column_that_is_not_a_whole_number_of_steps() {
    let narrow = Layout::reflowed(PhysicalSize::new(930, 1800), RETINA);
    let columns = usize::from(narrow.columns());

    assert_eq!(columns, 60);
    assert_ne!((columns - 1) % 16, 0);
    assert!((centring(&narrow, Some(columns - 1))).abs() < f32::EPSILON);
    assert!(
        (centring(&narrow, Some(16)) - f32::from(narrow.columns() - 16) / 2.0).abs() < f32::EPSILON
    );
}
