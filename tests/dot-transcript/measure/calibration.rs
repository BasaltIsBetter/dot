use super::*;

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

#[test]
fn before_any_reply_a_token_is_worth_two_point_three_characters() {
    let calibration = Calibration::default();

    assert!(close(calibration.tokens_for(23), 10.0));
    assert!(close(calibration.tokens_for(0), 0.0));
}

#[test]
fn learns_from_the_replies_of_the_conversation() {
    let mut calibration = Calibration::default();

    calibration.learn(1_000, 500);
    let after_one = calibration.tokens_for(10);
    calibration.learn(1_000, 300);

    assert!(close(after_one, 5.0));
    assert!(close(calibration.tokens_for(10), 4.0));
}

#[test]
fn a_reply_that_showed_or_wrote_nothing_teaches_nothing() {
    let mut calibration = Calibration::default();

    calibration.learn(0, 300);
    calibration.learn(400, 0);

    assert_eq!(calibration, Calibration::default());
}

#[test]
fn turns_counts_into_floats() {
    assert!(close(to_f64(0), 0.0));
    assert!(close(to_f64(1_234), 1_234.0));
}
