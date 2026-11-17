use super::*;

const RETINA: f64 = 2.0;
const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

fn layout() -> Layout {
    Layout::new(WINDOW, RETINA)
}

#[test]
fn sizes_the_text_at_13_on_20_points_on_a_retina_screen() {
    assert!((layout().font_size() - 26.0).abs() < f32::EPSILON);
    assert!((layout().line_height() - 40.0).abs() < f32::EPSILON);
}

#[test]
fn centres_the_column_in_the_window() {
    let column_width = 64.0 * 26.0 * 0.6;
    let block_height = 11.0 * 40.0;

    assert!((layout().left() - ((2880.0 - column_width) / 2.0_f32).floor()).abs() < 1.0);
    assert!((layout().line_top(0) - (1800.0 - block_height) / 2.0).abs() < 1.0);
}

#[test]
fn stacks_lines_one_line_height_apart() {
    let layout = layout();

    assert!((layout.line_top(3) - layout.line_top(2) - 40.0).abs() < f32::EPSILON);
}

#[test]
fn never_starts_outside_a_small_window() {
    let layout = Layout::new(PhysicalSize::new(100, 50), RETINA);

    assert!(layout.left().abs() < f32::EPSILON);
    assert!(layout.line_top(0).abs() < f32::EPSILON);
}

#[test]
fn finds_the_line_under_the_pointer() {
    let layout = layout();
    let x = f64::from(layout.left()) + 10.0;
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
    let left = f64::from(layout.left());
    let right = left + 64.0 * 26.0 * 0.6;

    assert_eq!(layout.line_at(PhysicalPosition::new(left - 1.0, y)), None);
    assert_eq!(layout.line_at(PhysicalPosition::new(left, y)), Some(3));
    assert_eq!(
        layout.line_at(PhysicalPosition::new(right - 1.0, y)),
        Some(3)
    );
    assert_eq!(layout.line_at(PhysicalPosition::new(right, y)), None);
    assert_eq!(
        layout.line_at(PhysicalPosition::new(
            left,
            f64::from(layout.line_top(Layout::ROWS))
        )),
        None
    );
}
