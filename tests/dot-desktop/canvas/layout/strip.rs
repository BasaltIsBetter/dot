use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::layout::tests::{RETINA, close, layout};

#[test]
fn a_reflowed_strip_keeps_its_full_height_in_a_small_window() {
    let small = Layout::reflowed(PhysicalSize::new(960, 640), RETINA);

    assert!(close(small.strip_top(), 595.0));
}

#[test]
fn lays_the_strip_along_the_bottom_of_the_window_with_small_text_across_its_middle() {
    let layout = layout();

    assert!(close(layout.width(), 2880.0));
    assert!(close(layout.strip_top(), 1755.0));
    assert!(close(layout.strip_middle(), 1777.0));
    assert!(close(layout.strip_text_top(), 1762.0));
}

#[test]
fn starts_the_strip_columns_after_sixteen_points() {
    let layout = layout();

    assert!(close(layout.strip_x(0), 25.0));
    assert!(close(layout.strip_x(10) - layout.strip_x(9), 10.56));
}

#[test]
fn counts_the_whole_columns_of_small_text_the_strip_holds() {
    let layout = layout();
    let columns = layout.strip_columns();

    assert_eq!(columns, 267);
    assert!(layout.strip_x(columns) <= 2880.0 - 25.0);
    assert!(layout.strip_x(columns + 1) > 2880.0 - 25.0);
}

#[test]
fn shrinks_the_strip_with_the_text() {
    let narrow = Layout::zoomed(PhysicalSize::new(800, 1800), RETINA);

    assert!(narrow.strip_top() > 1800.0 - 64.0);
    assert!(narrow.strip_x(narrow.strip_columns()) <= 800.0);
}

#[test]
fn keeps_the_strip_inside_a_tiny_window() {
    let tiny = Layout::zoomed(PhysicalSize::new(10, 10), RETINA);
    let empty = Layout::zoomed(PhysicalSize::new(0, 0), 1.0);

    assert!(close(tiny.strip_top(), 0.0));
    assert_eq!(tiny.strip_columns(), 0);
    assert_eq!(empty.strip_columns(), 0);
}

#[test]
fn has_room_for_the_middle_zone_past_six_hundred_and_forty_points() {
    let wide = Layout::reflowed(PhysicalSize::new(1026, 800), RETINA);
    let narrow = Layout::reflowed(PhysicalSize::new(1024, 800), RETINA);

    assert!(layout().has_room_for_the_middle_zone());
    assert!(wide.has_room_for_the_middle_zone());
    assert!(!narrow.has_room_for_the_middle_zone());
}

#[test]
fn sizes_the_gauge_and_the_zone_rules_in_whole_pixels() {
    let retina = layout();
    let half = Layout::zoomed(PhysicalSize::new(100, 50), 1.0);

    assert!(close(Layout::HAIRLINE, 1.0));
    assert_eq!(retina.gauge_size(), [80.0, 8.0]);
    assert!(close(retina.zone_rule_height(), 22.0));
    let [width, height] = half.gauge_size();
    assert!(width >= 1.0 && height >= 1.0);
    assert!(half.zone_rule_height() >= 1.0);
}
