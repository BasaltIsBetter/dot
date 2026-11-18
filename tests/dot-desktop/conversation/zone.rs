use winit::dpi::PhysicalSize;

use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
const LINE: f32 = 40.0;

fn zones(draft_rows: usize) -> (Zones, Layout) {
    let layout = Layout::new(WINDOW, RETINA);
    (Zones::new(&layout, draft_rows), layout)
}

#[test]
fn sits_the_prompter_just_above_the_status_bar() {
    let (zones, layout) = zones(1);

    assert_eq!(zones.prompter_rows, 1);
    assert!((zones.prompter.bottom - (layout.strip_top() - 24.0)).abs() < f32::EPSILON);
    assert!((zones.prompter.top - (zones.prompter.bottom - LINE)).abs() < f32::EPSILON);
}

#[test]
fn draws_the_rule_between_the_transcript_and_the_prompter() {
    let (zones, _) = zones(1);

    assert!((zones.rule - (zones.prompter.top - 24.0)).abs() < f32::EPSILON);
    assert!((zones.transcript.bottom - (zones.rule - 24.0)).abs() < f32::EPSILON);
    assert!((zones.transcript.top - 64.0).abs() < f32::EPSILON);
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
    let layout = Layout::new(PhysicalSize::new(960, 400), 1.0);

    let zones = Zones::new(&layout, 20);

    let between = zones.prompter.bottom - zones.transcript.top;
    let prompter = zones.prompter.bottom - zones.prompter.top;
    assert!(zones.prompter_rows < 8);
    assert!(prompter <= between / 2.0, "{prompter} of {between}");
}

#[test]
fn shows_at_least_one_prompter_row_in_the_smallest_window() {
    let layout = Layout::new(PhysicalSize::new(480, 40), 1.0);

    let zones = Zones::new(&layout, 5);

    assert_eq!(zones.prompter_rows, 1);
    assert!(zones.transcript.rows(layout.line_height()).abs() < f64::EPSILON);
}

#[test]
fn tells_which_area_is_under_the_pointer() {
    let (zones, layout) = zones(1);

    assert_eq!(zones.area_at(0.0), Some(Area::Transcript));
    assert_eq!(zones.area_at(zones.rule - 1.0), Some(Area::Transcript));
    assert_eq!(zones.area_at(zones.rule), Some(Area::Prompter));
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

    assert!((band.rows(LINE) - 5.0).abs() < f64::EPSILON);
    assert!(band.rows(0.0).abs() < f64::EPSILON);
    assert!(
        Band {
            top: 300.0,
            bottom: 100.0
        }
        .rows(LINE)
        .abs()
            < f64::EPSILON
    );
    assert!((band.row_top(1.5, LINE) - 160.0).abs() < f32::EPSILON);
}
