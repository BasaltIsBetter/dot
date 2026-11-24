use super::*;

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

#[test]
fn a_status_knows_nothing_of_the_folder_until_it_is_read() {
    let status = SessionStatus::default();

    assert_eq!((status.project, status.branch), (None, None));
}
