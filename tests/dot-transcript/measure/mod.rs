use proptest::prelude::*;

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

const CHARACTERS_OF_TEN_TOKENS: usize = 23;

fn calibration() -> Calibration {
    Calibration::default()
}

#[test]
fn shows_nothing_measured_before_claude_writes() {
    let sent = Instant::now();
    let meter = Meter::new(sent);

    let measures = meter.measures(after(sent, 700));

    assert_eq!(measures.tokens, 0);
    assert_eq!(measures.rate, None);
    assert_eq!(measures.elapsed, Duration::from_millis(700));
    assert_eq!(measures.spark, Spark::default());
    assert!(meter.is_live());
}

#[test]
fn counts_the_tokens_of_what_claude_shows_and_their_rate_over_the_last_second() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);

    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 300));
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 800));

    assert_eq!(meter.measures(after(sent, 800)).tokens, 20);
    assert_eq!(meter.measures(after(sent, 800)).rate, Some(20));
    assert_eq!(meter.measures(after(sent, 1_299)).rate, Some(20));
    assert_eq!(meter.measures(after(sent, 1_300)).rate, Some(10));
    assert_eq!(meter.measures(after(sent, 1_800)).rate, Some(0));
    assert_eq!(meter.counted(), (46, None));
}

#[test]
fn ignores_an_empty_piece() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);

    meter.hear(0, &calibration(), after(sent, 300));

    assert_eq!(meter.measures(after(sent, 400)).rate, None);
}

#[test]
fn samples_the_rate_every_fifth_of_a_second_since_the_first_piece() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(
        CHARACTERS_OF_TEN_TOKENS * 4,
        &calibration(),
        after(sent, 100),
    );

    let spark = meter.measures(after(sent, 1_050)).spark;

    let mut expected = [0; SPARK_COLUMNS];
    expected[SPARK_COLUMNS - 4..].copy_from_slice(&[2, 2, 2, 2]);
    assert_eq!(spark.cells, expected);
    assert_eq!(spark.flash, heat_of_a_column_sampled(150));
}

#[test]
fn samples_the_rate_where_it_was_at_each_fifth_of_a_second() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    let forty_tokens = CHARACTERS_OF_TEN_TOKENS * 4;
    meter.hear(forty_tokens, &calibration(), after(sent, 100));
    meter.hear(forty_tokens, &calibration(), after(sent, 1_500));

    let spark = meter.measures(after(sent, 2_050)).spark;

    let mut expected = [0; SPARK_COLUMNS];
    expected[SPARK_COLUMNS - 9..].copy_from_slice(&[2, 2, 2, 2, 0, 0, 2, 2, 2]);
    assert_eq!(spark.cells, expected);
}

#[test]
fn flashes_no_column_before_the_first_sample() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 100));

    let spark = meter.measures(after(sent, 250)).spark;

    assert_eq!(spark.flash, 0);
    assert_eq!(spark.cells, [0; SPARK_COLUMNS]);
}

fn heat_of_a_column_sampled(milliseconds_ago: u64) -> u8 {
    let sampled = Instant::now();
    Spark::of(&[40.0], Some(sampled), after(sampled, milliseconds_ago)).flash
}

#[test]
fn ends_with_the_tokens_claude_counted_and_its_rate_over_all_of_its_writing() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 500));

    meter.end(Some(300), after(sent, 2_500));

    let measures = meter.measures(after(sent, 9_000));
    assert!(!meter.is_live());
    assert_eq!(measures.tokens, 300);
    assert_eq!(measures.rate, Some(150));
    assert_eq!(measures.elapsed, Duration::from_millis(2_500));
    assert_eq!(measures.spark.flash, 0);
    assert_eq!(meter.counted(), (23, Some(300)));
}

#[test]
fn keeps_its_estimate_when_claude_counted_nothing() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 500));

    meter.end(Some(0), after(sent, 1_500));

    assert_eq!(meter.measures(after(sent, 1_500)).tokens, 10);
    assert_eq!(meter.measures(after(sent, 1_500)).rate, Some(10));
    assert_eq!(meter.counted(), (23, None));
}

#[test]
fn a_reply_ended_before_anything_was_written_has_no_rate() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);

    meter.end(None, after(sent, 400));

    assert_eq!(meter.measures(after(sent, 900)).rate, None);
    assert_eq!(
        meter.measures(after(sent, 900)).elapsed,
        Duration::from_millis(400)
    );
}

#[test]
fn hears_nothing_and_ends_once_after_the_end() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.end(Some(12), after(sent, 100));

    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 200));
    meter.end(Some(99), after(sent, 300));

    assert_eq!(meter.measures(after(sent, 400)).tokens, 12);
    assert_eq!(
        meter.measures(after(sent, 400)).elapsed,
        Duration::from_millis(100)
    );
}

#[test]
fn the_spark_stops_where_the_reply_ended() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS * 4, &calibration(), sent);
    meter.end(None, after(sent, 400));

    let at_the_end = meter.measures(after(sent, 400)).spark;
    let much_later = meter.measures(after(sent, 60_000)).spark;

    assert_eq!(at_the_end, much_later);
    assert_eq!(much_later.cells[SPARK_COLUMNS - 2..], [2, 2]);
}

#[test]
fn forgets_the_pieces_older_than_its_spark_needs() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), sent);

    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 6_001));

    assert_eq!(meter.arrivals.len(), 1);
    assert_eq!(meter.measures(after(sent, 6_001)).tokens, 20);
}

#[test]
fn keeps_a_piece_exactly_as_old_as_its_spark_needs() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);
    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), sent);

    meter.hear(CHARACTERS_OF_TEN_TOKENS, &calibration(), after(sent, 6_000));

    assert_eq!(meter.arrivals.len(), 2);
}

#[test]
fn changes_every_tenth_of_a_second_while_claude_writes() {
    let sent = Instant::now();
    let mut meter = Meter::new(sent);

    assert_eq!(meter.next_change(sent), Some(after(sent, 100)));
    assert_eq!(meter.next_change(after(sent, 250)), Some(after(sent, 300)));
    assert_eq!(meter.next_change(after(sent, 300)), Some(after(sent, 400)));

    meter.end(None, after(sent, 350));

    assert_eq!(meter.next_change(after(sent, 350)), None);
}

proptest! {
    #[test]
    fn the_spark_never_lights_more_than_its_cells(
        pieces in prop::collection::vec((1_usize..400, 0_u64..300), 0..60),
        looked in 0_u64..20_000,
    ) {
        let sent = Instant::now();
        let mut meter = Meter::new(sent);
        let mut at = sent;
        for (characters, wait) in pieces {
            at += Duration::from_millis(wait);
            meter.hear(characters, &calibration(), at);
        }

        let measures = meter.measures(at + Duration::from_millis(looked));

        prop_assert!(measures.spark.cells.iter().all(|&cells| cells <= SPARK_CELLS));
        prop_assert!(meter.arrivals.len() <= 60);
    }
}

#[test]
fn adds_tokens_counted_elsewhere_to_its_own_without_learning_from_them() {
    let start = Instant::now();
    let calibration = Calibration::default();
    let mut meter = Meter::new(start);
    meter.hear(23, &calibration, start + Duration::from_millis(100));

    meter.hear_tokens(40, start + Duration::from_millis(200));
    meter.hear_tokens(0, start + Duration::from_millis(300));
    let live = meter.measures(start + Duration::from_millis(400));
    meter.end(Some(12), start + Duration::from_millis(1_100));
    meter.hear_tokens(99, start + Duration::from_millis(1_200));

    assert_eq!(live.tokens, 10 + 40);
    assert_eq!(
        meter.measures(start + Duration::from_secs(5)).tokens,
        12 + 40
    );
    assert_eq!(meter.counted(), (23, Some(12)));
}

#[test]
fn tokens_counted_elsewhere_start_the_rate_and_the_spark() {
    let start = Instant::now();
    let mut meter = Meter::new(start);

    meter.hear_tokens(20, start + Duration::from_millis(500));

    let measures = meter.measures(start + Duration::from_millis(900));
    assert_eq!(measures.rate, Some(20));
    assert!(measures.spark.rates.iter().any(|&rate| rate > 0));
}
