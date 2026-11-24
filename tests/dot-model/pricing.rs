use super::*;

fn usage(json: &str) -> Usage {
    serde_json::from_str(json).unwrap()
}

#[test]
fn prices_opus_5_5_input_and_output() {
    let usage = usage(r#"{"input_tokens": 1000000, "output_tokens": 100000}"#);

    assert_eq!(
        cost("claude-opus-5-5", &usage),
        Cost::from_micro_dollars(6_000_000)
    );
}

#[test]
fn prices_cache_reads_and_writes_by_their_duration() {
    let usage = usage(
        r#"{"cache_read_input_tokens": 1000000, "cache_creation_input_tokens": 300000,
            "cache_creation": {"ephemeral_5m_input_tokens": 100000, "ephemeral_1h_input_tokens": 200000}}"#,
    );

    let expected = 200_000 + 100_000 * 5 + 200_000 * 8;
    assert_eq!(
        cost("claude-opus-5-5", &usage),
        Cost::from_micro_dollars(expected)
    );
}

#[test]
fn prices_cache_writes_for_five_minutes_without_breakdown() {
    let usage = usage(r#"{"cache_creation_input_tokens": 1000}"#);

    assert_eq!(
        cost("claude-opus-5-5", &usage),
        Cost::from_micro_dollars(5_000)
    );
}

#[test]
fn prices_a_fallback_model_at_its_own_rates() {
    let usage =
        usage(r#"{"input_tokens": 1000, "output_tokens": 1000, "cache_read_input_tokens": 1000}"#);

    assert_eq!(
        cost("claude-opus-5", &usage),
        Cost::from_micro_dollars(30_500)
    );
    assert_eq!(
        cost("claude-opus-4-8", &usage),
        Cost::from_micro_dollars(30_500)
    );
    assert_eq!(
        cost("claude-opus-5-5", &usage),
        Cost::from_micro_dollars(24_200)
    );
}

#[test]
fn prices_an_unknown_model_like_opus_5_5() {
    let usage = usage(r#"{"output_tokens": 1}"#);

    assert_eq!(
        cost("claude-future", &usage),
        cost("claude-opus-5-5", &usage)
    );
}

#[test]
fn rounds_to_the_nearest_micro_dollar() {
    assert_eq!(
        cost(
            "claude-opus-5-5",
            &usage(r#"{"cache_read_input_tokens": 2}"#)
        ),
        Cost::from_micro_dollars(0)
    );
    assert_eq!(
        cost(
            "claude-opus-5-5",
            &usage(r#"{"cache_read_input_tokens": 3}"#)
        ),
        Cost::from_micro_dollars(1)
    );
}

#[test]
fn never_overflows_on_absurd_counts() {
    let usage = usage(&format!(
        r#"{{"input_tokens": {}, "output_tokens": {}}}"#,
        u64::MAX,
        u64::MAX
    ));

    assert_eq!(
        cost("claude-opus-5-5", &usage),
        Cost::from_micro_dollars(u64::MAX / NANO_DOLLARS_PER_MICRO_DOLLAR)
    );
}

#[test]
fn costs_nothing_without_tokens() {
    assert_eq!(cost("claude-opus-5-5", &Usage::default()), Cost::default());
}
#[test]
fn bills_a_reply_what_the_running_total_grew_by() {
    let mut billed = 0;

    let first = billed_since(&mut billed, Cost::from_micro_dollars(50_000));
    let second = billed_since(&mut billed, Cost::from_micro_dollars(80_000));
    let unchanged = billed_since(&mut billed, Cost::from_micro_dollars(80_000));

    assert_eq!(
        (
            first.micro_dollars(),
            second.micro_dollars(),
            unchanged.micro_dollars()
        ),
        (50_000, 30_000, 0)
    );
    assert_eq!(billed, 80_000);
}

#[test]
fn bills_a_total_that_started_again_in_full() {
    let mut billed = 80_000;

    let restarted = billed_since(&mut billed, Cost::from_micro_dollars(20_000));

    assert_eq!(restarted.micro_dollars(), 20_000);
    assert_eq!(billed, 20_000);
}

#[test]
fn prices_each_model_dot_offers() {
    let usage = usage(
        r#"{"input_tokens": 1000, "output_tokens": 100, "cache_read_input_tokens": 1000,
            "cache_creation_input_tokens": 1000,
            "cache_creation": {"ephemeral_5m_input_tokens": 1000}}"#,
    );
    let prices = [
        ("claude-haiku-5-5", 100 + 50 + 10 + 125),
        ("claude-sonnet-5-5", 2_000 + 1_000 + 100 + 2_500),
        ("claude-opus-5-5", 4_000 + 2_000 + 200 + 5_000),
        ("claude-fable-5-2", 8_000 + 4_000 + 200 + 10_000),
    ];

    for (model, micro_dollars) in prices {
        assert_eq!(
            cost(model, &usage),
            Cost::from_micro_dollars(micro_dollars),
            "{model}"
        );
    }
}

#[test]
fn prices_an_hour_of_cache_for_each_model() {
    let usage = usage(
        r#"{"cache_creation_input_tokens": 1000000,
            "cache_creation": {"ephemeral_1h_input_tokens": 1000000}}"#,
    );

    assert_eq!(
        cost("claude-sonnet-5-5", &usage),
        Cost::from_micro_dollars(4_000_000)
    );
    assert_eq!(cost("claude-fable-5-2", &usage).micro_dollars(), 16_000_000);
}

#[test]
fn prices_haiku_higher_once_the_prompt_passes_a_hundred_thousand_tokens() {
    let at_the_limit = usage(
        r#"{"input_tokens": 40000, "cache_read_input_tokens": 60000, "output_tokens": 1000}"#,
    );
    let beyond = usage(
        r#"{"input_tokens": 40001, "cache_read_input_tokens": 60000, "output_tokens": 1000}"#,
    );

    assert_eq!(
        cost("claude-haiku-5-5", &at_the_limit),
        Cost::from_micro_dollars(4_000 + 600 + 500)
    );
    assert_eq!(
        cost("claude-haiku-5-5", &beyond),
        Cost::from_micro_dollars(20_001 + 3_000 + 2_500)
    );
}

#[test]
fn counts_cache_writes_in_the_length_of_a_haiku_prompt() {
    let written = usage(
        r#"{"input_tokens": 1, "cache_creation_input_tokens": 100000,
            "cache_creation": {"ephemeral_5m_input_tokens": 100000}}"#,
    );

    assert_eq!(
        cost("claude-haiku-5-5", &written),
        Cost::from_micro_dollars(1 + 62_500)
    );
}

#[test]
fn keeps_the_long_prompt_price_to_haiku() {
    let long = usage(r#"{"input_tokens": 200000}"#);

    assert_eq!(
        cost("claude-sonnet-5-5", &long),
        Cost::from_micro_dollars(400_000)
    );
}
