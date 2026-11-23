use winit::dpi::PhysicalSize;

use super::*;

const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);

#[test]
fn takes_the_teleprompter_sizes_at_the_interface_zoom() {
    let layout = Layout::reflowed(WINDOW, 2.0);

    assert!((layout.font_size_of(TextSize::Regular) - 13.0 * 1.6).abs() < 1e-4);
    assert!((layout.font_size_of(TextSize::Small) - 11.0 * 1.6).abs() < 1e-4);
    assert!((layout.font_size_of(TextSize::Tiny) - 9.0 * 1.6).abs() < 1e-4);
    assert!((layout.line_height_of(TextSize::Regular) - 22.0 * 1.6).abs() < 1e-4);
    assert!((layout.line_height_of(TextSize::Small) - 18.0 * 1.6).abs() < 1e-4);
    assert!((layout.line_height_of(TextSize::Tiny) - 12.0 * 1.6).abs() < 1e-4);
}

#[test]
fn advances_each_size_by_its_own_width() {
    let layout = Layout::reflowed(WINDOW, 2.0);

    assert!((layout.advance_of(TextSize::Regular) - layout.advance()).abs() < f32::EPSILON);
    assert!((layout.advance_of(TextSize::Small) - 11.0 * 1.6 * 0.6).abs() < 1e-4);
    assert!(layout.advance_of(TextSize::Tiny) < layout.advance_of(TextSize::Small));
}

#[test]
fn centres_a_text_in_its_box_to_the_pixel() {
    let layout = Layout::reflowed(WINDOW, 2.0);
    let line = layout.line_height_of(TextSize::Small);

    let top = layout.text_top(TextSize::Small, 100.0, line + 10.0);

    assert!((top - 105.0).abs() < f32::EPSILON);
    assert!((layout.text_top(TextSize::Small, 100.0, line + 3.0) - 102.0).abs() < f32::EPSILON);
}
