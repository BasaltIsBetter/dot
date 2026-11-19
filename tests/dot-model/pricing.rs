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
