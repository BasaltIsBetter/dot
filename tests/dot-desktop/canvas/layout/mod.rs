use super::*;

pub(super) const RETINA: f64 = 2.0;
pub(super) const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
const COLUMN_WIDTH: f32 = 64.0 * 20.8 * 0.6;

pub(super) fn layout() -> Layout {
    Layout::zoomed(WINDOW, RETINA)
}

pub(super) fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn sizes_the_text_at_eight_tenths_of_13_on_20_points_on_a_retina_screen() {
    assert!(close(layout().font_size(), 20.8));
    assert!(close(layout().line_height(), 32.0));
}

#[test]
fn centres_the_column_across_the_window() {
    let layout = layout();
    let left_margin = layout.x(0);
    let right_margin = 2880.0 - layout.x(Layout::GRID_COLUMNS);

    assert!(close(
        left_margin,
        ((2880.0 - COLUMN_WIDTH) / 2.0_f32).floor()
    ));
    assert!((left_margin - right_margin).abs() <= 2.0);
}

#[test]
fn centres_the_main_rows_down_the_window() {
    let main_height = 8.0 * 32.0;

    assert!(close(
        layout().line_top(0),
        ((1800.0 - main_height) / 2.0_f32).floor()
    ));
}

#[test]
fn places_columns_and_lines_on_the_grid() {
    let layout = layout();

    assert!(close(layout.x(3) - layout.x(2), 12.48));
    assert!(close(layout.line_top(3) - layout.line_top(2), 32.0));
}

#[test]
fn shrinks_everything_to_fit_a_narrow_window() {
    let narrow = Layout::zoomed(PhysicalSize::new(800, 1800), RETINA);

    let width = narrow.x(Layout::GRID_COLUMNS) - narrow.x(0);

    assert!(narrow.font_size() < 20.8);
    assert!(close(width, 800.0 - 2.0 * 38.4));
    assert!(close(narrow.x(0), 38.4_f32.floor()));
}

#[test]
fn shrinks_everything_to_fit_a_short_window() {
    let short = Layout::zoomed(PhysicalSize::new(2880, 500), RETINA);

    let below_centre = 14.0 * short.line_height();

    assert!(short.font_size() < 20.8);
    assert!(close(below_centre, 500.0 - 2.0 * 38.4));
    assert!(short.line_top(Layout::ROWS) <= 500.0);
}

#[test]
fn never_shrinks_below_half_size() {
    let tiny = Layout::zoomed(PhysicalSize::new(100, 50), RETINA);

    assert!(close(tiny.font_size(), 10.4));
    assert!(close(tiny.x(0), 0.0));
    assert!(close(tiny.line_top(0), 0.0));
}

#[test]
fn keeps_full_size_in_a_large_window() {
    let large = Layout::zoomed(PhysicalSize::new(5120, 2880), RETINA);

    assert!(close(large.font_size(), 20.8));
}

#[test]
fn a_zoomed_grid_always_has_sixty_four_columns() {
    assert_eq!(layout().columns(), Layout::GRID_COLUMNS);
    assert_eq!(
        Layout::zoomed(PhysicalSize::new(100, 50), RETINA).columns(),
        Layout::GRID_COLUMNS
    );
}

#[test]
fn a_reflowed_column_leans_left_of_the_centre_beside_the_sessions_column() {
    let wide = Layout::reflowed(WINDOW, RETINA);
    let width = 96.0 * 12.48;
    let free = 2880.0 - width;
    let shift = free * 200.0 / 640.0 - free / 2.0;

    assert_eq!(wide.room(), Room::Railed);
    assert_eq!(wide.columns(), Layout::WIDEST_COLUMN);
    assert!(close(wide.x(0), ((free / 2.0_f32).floor() + shift).floor()));
    assert!(close(wide.x(96) - wide.x(0), width));
}

#[test]
fn a_reflowed_column_is_centred_in_a_wide_window_without_sessions_column() {
    let wide = Layout::reflowed(PhysicalSize::new(1_600, 1_400), RETINA);
    let width = f32::from(wide.columns()) * 12.48;

    assert_eq!(wide.room(), Room::Wide);
    assert!(close(wide.x(0), ((1_600.0 - width) / 2.0_f32).floor()));
}

#[test]
fn a_reflowed_column_keeps_the_text_full_size_on_its_teleprompter_rows() {
    let smallest = Layout::reflowed(PhysicalSize::new(960, 640), RETINA);

    assert!(close(smallest.font_size(), 20.8));
    assert!(close(smallest.line_height(), 35.2));
    assert!(close(layout().line_height(), 32.0));
}

#[test]
fn a_reflowed_column_narrows_to_the_window_minus_its_margins() {
    let narrow = Layout::reflowed(PhysicalSize::new(960, 640), RETINA);

    assert_eq!(narrow.room(), Room::Narrow);
    assert_eq!(narrow.columns(), 64);
    assert!(narrow.x(0) >= 76.8);
    assert!(narrow.x(narrow.columns()) <= 960.0 - 76.8);
    assert!(narrow.x(narrow.columns() + 1) > 960.0 - 76.8);
}

#[test]
fn a_reflowed_column_never_narrows_below_twenty_four_columns() {
    let tiny = Layout::reflowed(PhysicalSize::new(100, 50), RETINA);
    let empty = Layout::reflowed(PhysicalSize::new(0, 0), 1.0);

    assert_eq!(tiny.columns(), Layout::NARROWEST_COLUMN);
    assert_eq!(empty.columns(), Layout::NARROWEST_COLUMN);
    assert!(close(tiny.x(0), 0.0));
}

#[test]
fn finds_the_line_under_the_pointer() {
    let layout = layout();
    let x = f64::from(layout.x(0)) + 10.0;
    let inside_line_three = f64::from(layout.line_top(3)) + 1.0;
    let just_above_line_three = f64::from(layout.line_top(3)) - 1.0;

    assert_eq!(
        layout.line_at(PhysicalPosition::new(x, inside_line_three)),
        Some(3)
    );
    assert_eq!(
        layout.line_at(PhysicalPosition::new(x, just_above_line_three)),
        Some(2)
    );
}

#[test]
fn finds_no_line_beside_or_below_the_column() {
    let layout = layout();
    let y = f64::from(layout.line_top(3)) + 1.0;
    let left = f64::from(layout.x(0));
    let right = f64::from(layout.x(Layout::GRID_COLUMNS));

    assert_eq!(layout.line_at(PhysicalPosition::new(left - 1.0, y)), None);
    assert_eq!(layout.line_at(PhysicalPosition::new(left, y)), Some(3));
    assert_eq!(
        layout.line_at(PhysicalPosition::new(right - 1.0, y)),
        Some(3)
    );
    assert_eq!(layout.line_at(PhysicalPosition::new(right + 0.5, y)), None);
    assert_eq!(
        layout.line_at(PhysicalPosition::new(
            left,
            f64::from(layout.line_top(Layout::ROWS)) + 0.5
        )),
        None
    );
}

#[test]
fn gives_a_pointer_on_a_boundary_to_the_line_below() {
    let layout = layout();
    let x = f64::from(layout.x(1));

    let boundary = f64::from(layout.line_top(4));

    assert_eq!(layout.line_at(PhysicalPosition::new(x, boundary)), Some(4));
}

#[test]
fn scales_points_to_whole_pixels() {
    assert!(close(layout().scaled(12.0), 19.0));
    assert!(close(
        Layout::zoomed(PhysicalSize::new(800, 1800), RETINA).scaled(12.0),
        17.0
    ));
}
