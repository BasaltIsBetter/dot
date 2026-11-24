use winit::dpi::PhysicalSize;

use super::*;

const RAILED: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);
const NARROW: PhysicalSize<u32> = PhysicalSize::new(1000, 700);
const RETINA: f64 = 2.0;

fn railed() -> Stage {
    Stage::of(&Layout::reflowed(RAILED, RETINA))
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn reads_on_a_line_across_the_middle_of_the_window() {
    let stage = railed();

    assert!(close(stage.row, 35.2));
    assert!(close(stage.centre, 700.0));
    assert!(close(stage.floor, 1355.0));
    assert_eq!(
        stage.band(),
        Band {
            top: 0.0,
            bottom: 1355.0
        }
    );
}

#[test]
fn leaves_room_for_the_gutter_and_the_sessions_column_when_wide() {
    let stage = railed();

    assert!(close(stage.column[0], 320.0), "{:?}", stage.column);
    assert!(close(stage.column[1], 320.0 + 78.0 * 12.48));
    let [gutter_left, gutter_right] = stage.gutter.unwrap();
    assert!(close(gutter_left, stage.column[1] + 26.0));
    assert!(close(gutter_right - gutter_left, 256.0));
    let [rail_left, rail_right] = stage.rail.unwrap();
    assert!(close(rail_right, 2000.0 - 38.0));
    assert!(close(rail_right - rail_left, 320.0));
}

#[test]
fn has_neither_gutter_nor_sessions_column_in_a_narrow_window() {
    let stage = Stage::of(&Layout::reflowed(NARROW, RETINA));

    assert_eq!(stage.gutter, None);
    assert_eq!(stage.rail, None);
}

#[test]
fn places_the_row_on_the_line_in_the_middle_and_the_others_around_it() {
    let stage = railed();
    let top = stage.top_row(10.0);

    assert!(close(stage.row_top(10, top), 682.0));
    assert!(close(stage.row_top(11, top), (682.4_f32 + 35.2).round()));
    assert!(close(stage.row_top(9, top), (682.4_f32 - 35.2).round()));
}

#[test]
fn finds_the_row_under_a_point_of_the_window() {
    let stage = railed();

    assert_eq!(stage.row_at(700.0, 10.0), Some(10));
    assert_eq!(stage.row_at(700.0 + 35.2, 10.0), Some(11));
    assert_eq!(stage.row_at(1355.0, 10.0), None);
    assert_eq!(stage.row_at(0.0, 0.0), None);
}

#[test]
fn counts_the_rows_that_fit_under_the_line_above_the_bottom_margin() {
    let lower = Stage::of(&Layout::reflowed(PhysicalSize::new(2000, 1350), RETINA));

    assert_eq!(railed().rows_below(), 16);
    assert_eq!(lower.rows_below(), 15);
}

#[test]
fn finds_the_middle_of_the_window_on_the_reading_line() {
    assert_eq!(railed().middle(), [1_000.0, 700.0]);
}

#[test]
fn draws_the_band_around_the_line_to_the_sessions_column() {
    let stage = railed();

    let band = stage.reading_band(0.0);

    assert!(close(band.top, 676.0));
    assert!(close(band.height, 48.0));
    assert!(close(band.left, 0.0));
    let [rail_left, _] = stage.rail.unwrap();
    assert!(close(band.width, rail_left - 51.0));
    assert_eq!(band.color, Paint::ink(Ink::Trace));
}

#[test]
fn ends_the_band_past_the_column_without_a_sessions_column() {
    let stage = Stage::of(&Layout::reflowed(NARROW, RETINA));

    assert!(close(stage.reading_band(0.0).width, stage.column[1] + 38.0));
}

#[test]
fn keeps_the_band_inside_the_window_where_it_follows_the_writing() {
    let stage = railed();

    assert!(close(
        stage.reading_band(2.0).top,
        (676.0_f32 + 70.4).round()
    ));
    assert!(close(stage.reading_band(100.0).top, 1274.0));
    assert!(close(stage.reading_band(-100.0).top, 11.0));
}

#[test]
fn draws_a_small_tick_at_the_left_of_the_band() {
    let stage = railed();
    let band = stage.reading_band(0.0);

    let tick = stage.tick(&band);

    assert!(close(tick.left, 26.0));
    assert!(close(tick.width, 26.0));
    assert!(close(tick.top, 700.0));
    assert!(close(tick.height, 1.0));
    assert_eq!(tick.color, Paint::ink(Ink::Receding));
}

#[test]
fn fades_the_conversation_to_black_above_the_status_bar() {
    let stage = railed();

    let veils = stage.veils();

    assert_eq!(veils.len(), 51);
    assert!(close(veils[0].top, 1304.0));
    assert!(close(veils[50].top, 1354.0));
    assert!(
        veils
            .iter()
            .all(|veil| close(veil.width, 2000.0) && close(veil.height, 1.0))
    );
    assert!(
        veils
            .windows(2)
            .all(|pair| pair[0].color.alpha <= pair[1].color.alpha)
    );
    assert!(veils[0].color.alpha < 10);
    assert!(veils[50].color.alpha > 245);
    assert!(veils.iter().all(|veil| veil.color.rgb() == [0, 0, 0]));
}

#[test]
fn gives_the_text_the_column_less_its_indent() {
    let layout = Layout::reflowed(RAILED, RETINA);

    assert_eq!(text_width(&layout).columns(), 76);
}

#[test]
fn the_sessions_column_is_always_at_its_widest_beside_its_reserve() {
    for width in [1_760, 2_000, 2_400, 3_200] {
        let stage = Stage::of(&Layout::reflowed(PhysicalSize::new(width, 1_400), 2.0));

        let [left, right] = stage.rail.unwrap();

        assert!(
            (right - left - 320.0).abs() < 0.01,
            "{width}: {left} {right}"
        );
        assert!((f32::from(u16::try_from(width).unwrap()) - right - 38.0).abs() < 0.01);
    }
}

#[test]
fn pins_the_rows_below_the_line_the_veils_and_the_exact_points() {
    let stage = Stage::of(&Layout::reflowed(PhysicalSize::new(2_880, 1_800), 2.0));

    let veils = stage.veils();

    assert_eq!(stage.rows_below(), 22);
    assert_eq!(veils.len(), 51);
    assert_eq!((veils[0].top, veils[0].color.alpha), (1_704.0, 3));
    assert_eq!((veils[50].top, veils[50].color.alpha), (1_754.0, 252));
    assert!((stage.exact_points(84.0) - 134.4).abs() < 0.001);
}

#[test]
fn draws_a_band_even_in_a_window_a_few_pixels_tall() {
    for height in [0, 1, 2, 10, 40] {
        let stage = Stage::of(&Layout::reflowed(PhysicalSize::new(2000, height), RETINA));

        for rows_away in [-100.0, 0.0, 100.0] {
            assert!(stage.reading_band(rows_away).top.is_finite(), "{height}");
        }
    }
}

#[test]
fn finds_the_rows_that_reach_into_the_window() {
    let top_of = |row: usize| f32::from(u16::try_from(row).unwrap()) * 10.0 - 25.0;

    assert_eq!(rows_in_sight(100, top_of, 10.0, 50.0), 2..8);
    assert_eq!(rows_in_sight(5, top_of, 10.0, 50.0), 2..5);
    assert_eq!(rows_in_sight(0, top_of, 10.0, 50.0), 0..0);
    assert_eq!(rows_in_sight(2, top_of, 10.0, 50.0), 2..2);
    assert_eq!(
        rows_in_sight(100, |row| top_of(row) + 1_000.0, 10.0, 50.0),
        0..0
    );
}
