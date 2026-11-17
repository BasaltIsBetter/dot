use super::*;

#[test]
fn opus_5_5_is_named_like_claude_code_names_it() {
    assert_eq!(Model::OPUS_5_5.id, "claude-opus-5-5");
    assert_eq!(Model::OPUS_5_5.name, "Opus 5.5");
}

#[test]
fn opus_5_5_reads_a_million_tokens() {
    assert_eq!(Model::OPUS_5_5.context_window, 1_000_000);
}

#[test]
fn keeps_a_cost_to_the_micro_dollar() {
    let cost = Cost::from_micro_dollars(2_400_001);

    assert_eq!(cost.micro_dollars(), 2_400_001);
    assert!(Cost::from_micro_dollars(1) > Cost::default());
}

#[test]
fn knows_no_limit_until_claude_reports_it() {
    assert_eq!(Limits::default().five_hours, None);
    assert_eq!(Limits::default().seven_days, None);
}

#[test]
fn a_new_session_has_consumed_nothing() {
    let usage = Usage::default();

    assert_eq!(usage.context_tokens, 0);
    assert_eq!((usage.lines_added, usage.lines_removed), (0, 0));
    assert_eq!(usage.cost.micro_dollars(), 0);
}
