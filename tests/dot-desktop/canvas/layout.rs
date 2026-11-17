use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);
const COLUMN_WIDTH: f32 = 64.0 * 26.0 * 0.6;

fn layout() -> Layout {
    Layout::new(WINDOW, RETINA)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn sizes_the_text_at_13_on_20_points_on_a_retina_screen() {
    assert!(close(layout().font_size(), 26.0));
    assert!(close(layout().line_height(), 40.0));
}

#[test]
fn centres_the_column_across_the_window() {
    let layout = layout();
    let left_margin = layout.x(0);
    let right_margin = 2880.0 - layout.x(Layout::COLUMNS);

    assert!(close(
        left_margin,
        ((2880.0 - COLUMN_WIDTH) / 2.0_f32).floor()
    ));
    assert!((left_margin - right_margin).abs() <= 2.0);
}

#[test]
fn centres_the_main_rows_down_the_window() {
    let main_height = 8.0 * 40.0;

    assert!(close(
        layout().line_top(0),
        ((1800.0 - main_height) / 2.0_f32).floor()
    ));
}

#[test]
fn places_columns_and_lines_on_the_grid() {
    let layout = layout();

    assert!(close(layout.x(3) - layout.x(2), 15.6));
    assert!(close(layout.line_top(3) - layout.line_top(2), 40.0));
}

#[test]
fn shrinks_everything_to_fit_a_narrow_window() {
    let narrow = Layout::new(PhysicalSize::new(800, 1800), RETINA);

    let width = narrow.x(Layout::COLUMNS) - narrow.x(0);

    assert!(narrow.font_size() < 26.0);
    assert!(close(width, 800.0 - 2.0 * 48.0));
    assert!(close(narrow.x(0), 48.0));
}

#[test]
fn shrinks_everything_to_fit_a_short_window() {
    let short = Layout::new(PhysicalSize::new(2880, 600), RETINA);

    let below_centre = 14.0 * short.line_height();

    assert!(short.font_size() < 26.0);
    assert!(close(below_centre, 600.0 - 2.0 * 48.0));
    assert!(short.line_top(Layout::ROWS) <= 600.0);
}

#[test]
fn never_shrinks_below_half_size() {
    let tiny = Layout::new(PhysicalSize::new(100, 50), RETINA);

    assert!(close(tiny.font_size(), 13.0));
    assert!(close(tiny.x(0), 0.0));
    assert!(close(tiny.line_top(0), 0.0));
}

#[test]
fn keeps_full_size_in_a_large_window() {
    let large = Layout::new(PhysicalSize::new(5120, 2880), RETINA);

    assert!(close(large.font_size(), 26.0));
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
    let right = f64::from(layout.x(Layout::COLUMNS));

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
