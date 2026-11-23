use std::time::{Duration, Instant};

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn lights_a_cell_per_eighteen_tokens_a_second_up_to_five() {
    assert_eq!(cells_for(0.0), 0);
    assert_eq!(cells_for(-3.0), 0);
    assert_eq!(cells_for(f64::NAN), 0);
    assert_eq!(cells_for(1.0), 1);
    assert_eq!(cells_for(26.9), 1);
    assert_eq!(cells_for(27.0), 2);
    assert_eq!(cells_for(36.0), 2);
    assert_eq!(cells_for(80.0), 4);
    assert_eq!(cells_for(81.0), 5);
    assert_eq!(cells_for(1_000.0), 5);
}

#[test]
fn draws_the_newest_rates_on_the_right_and_leaves_the_rest_dark() {
    let now = Instant::now();

    let spark = Spark::of(&[18.0, 36.0], None, now);

    let mut expected = [0; SPARK_COLUMNS];
    expected[SPARK_COLUMNS - 2] = 1;
    expected[SPARK_COLUMNS - 1] = 2;
    assert_eq!(spark.cells, expected);
    let mut rates = [0; SPARK_COLUMNS];
    rates[SPARK_COLUMNS - 2] = 18;
    rates[SPARK_COLUMNS - 1] = 36;
    assert_eq!(spark.rates, rates);
    assert_eq!(spark.flash, 0);
}

#[test]
fn keeps_only_the_last_twenty_rates() {
    let rates: Vec<f64> = (0..25)
        .map(|index| if index < 5 { 90.0 } else { 18.0 })
        .collect();

    let spark = Spark::of(&rates, None, Instant::now());

    assert_eq!(spark.cells, [1; SPARK_COLUMNS]);
}

#[test]
fn the_newest_column_flashes_orange_then_cools_in_four_hundred_and_fifty_milliseconds() {
    let sampled = Instant::now();
    let flash =
        |milliseconds| Spark::of(&[36.0], Some(sampled), after(sampled, milliseconds)).flash;

    assert_eq!(flash(0), u8::MAX);
    assert_eq!(flash(119), u8::MAX);
    assert_eq!(flash(120), u8::MAX);
    assert_eq!(flash(345), 80);
    assert_eq!(flash(570), 0);
}

#[test]
fn only_a_lit_newest_column_flashes() {
    let sampled = Instant::now();

    let lit = Spark::of(&[36.0], Some(sampled), sampled);
    let dark = Spark::of(&[0.0], Some(sampled), sampled);
    let cooled = Spark::of(&[36.0], Some(sampled), after(sampled, 600));

    assert!(lit.is_flashing());
    assert!(!dark.is_flashing());
    assert!(!cooled.is_flashing());
}

#[test]
fn keeps_each_rate_rounded_within_its_range() {
    assert_eq!(to_rate(12.4), 12);
    assert_eq!(to_rate(12.5), 13);
    assert_eq!(to_rate(-4.0), 0);
    assert_eq!(to_rate(f64::NAN), 0);
    assert_eq!(to_rate(1e9), u16::MAX);
}
