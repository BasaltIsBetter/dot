use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::Layout;

const RAILED: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);

fn layout(size: PhysicalSize<u32>) -> Layout {
    Layout::reflowed(size, 2.0)
}

#[test]
fn lights_a_mini_cell_for_every_twenty_nine_tokens_a_second_up_to_three() {
    assert_eq!(lit_cells(0), 0);
    assert_eq!(lit_cells(1), 1);
    assert_eq!(lit_cells(43), 1);
    assert_eq!(lit_cells(44), 2);
    assert_eq!(lit_cells(73), 3);
    assert_eq!(lit_cells(u16::MAX), 3);
}

#[test]
fn draws_the_mini_spark_with_sixteen_columns_of_three_cells_the_newest_hot_while_live() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let mut rates = [0; SPARK_COLUMNS];
    rates[SPARK_COLUMNS - 1] = 90;

    let live = mini_spark(&rates, [100.0, 100.0], &stage, Some(Ink::Live));
    let settled = mini_spark(&rates, [100.0, 100.0], &stage, None);

    assert_eq!(live.len(), 48);
    assert_eq!(live[45].color, Paint::ink(Ink::Live));
    assert_eq!(settled[45].color, Paint::ink(Ink::Receding));
    assert_eq!(live[0].color, Paint::ink(Ink::Faint));
    assert_eq!(live[1].color, Paint::ink(Ink::Trace));
    let right = live
        .iter()
        .map(|cell| cell.left + cell.width)
        .fold(f32::MIN, f32::max);
    assert!((right - (100.0 + stage.exact_points(84.0))).abs() <= 1.0);
}
