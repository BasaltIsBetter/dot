use proptest::prelude::*;

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.001
}

fn gliding(from: f32, to: f32, start: Instant) -> Glide {
    let mut glide = Glide::at_rest(from, SETTLE);
    glide.toward(to, start);
    glide
}

#[test]
fn eases_out_from_nothing_to_everything() {
    assert!(ease_out(0.0).abs() < 1e-6);
    assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
    assert!(ease_out(0.5) > 0.5);
}

#[test]
fn follows_the_css_curves_of_spaceship() {
    for (progress, settle, pin) in [
        (0.1, 0.160_572_154_2, 0.396_693_549_0),
        (0.25, 0.378_138_130_8, 0.767_284_300_7),
        (0.5, 0.684_643_187_4, 0.946_079_461_5),
        (0.75, 0.906_535_349_3, 0.991_108_187_3),
    ] {
        assert!((ease_out(progress) - settle).abs() < 1e-6, "{progress}");
        assert!((PIN.curve.at(progress) - pin).abs() < 1e-6, "{progress}");
    }
    assert!(PIN.curve.at(0.0).abs() < 1e-6);
    assert!((PIN.curve.at(1.0) - 1.0).abs() < 1e-6);
}

proptest! {
    #[test]
    fn eases_out_only_forwards(earlier in 0.0_f64..1.0, later_by in 0.0_f64..1.0) {
        let later = (earlier + later_by).min(1.0);
        prop_assert!(ease_out(earlier) <= ease_out(later) + 1e-9);
        prop_assert!(PIN.curve.at(earlier) <= PIN.curve.at(later) + 1e-9);
    }
}

#[test]
fn a_glide_at_rest_stays_where_it_is() {
    let glide = Glide::at_rest(12.5, SETTLE);

    assert!(close(glide.position(Instant::now()), 12.5));
    assert!(close(glide.target(), 12.5));
    assert!(!glide.is_moving(Instant::now()));
}

#[test]
fn glides_from_where_it_was_to_its_target_in_three_hundred_milliseconds() {
    let start = Instant::now();
    let glide = gliding(40.0, 10.0, start);

    assert!(close(glide.position(start), 40.0));
    let halfway = glide.position(after(start, 150));
    assert!(halfway < 25.0 && halfway > 10.0, "{halfway}");
    assert!(glide.is_moving(after(start, 299)));
    assert!(close(glide.position(after(start, 300)), 10.0));
    assert!(!glide.is_moving(after(start, 300)));
    assert!(close(glide.target(), 10.0));
}

#[test]
fn keeps_gliding_when_aimed_at_the_same_target() {
    let start = Instant::now();
    let mut glide = gliding(40.0, 10.0, start);
    let before = glide.position(after(start, 150));

    glide.toward(10.0, after(start, 150));

    assert!(close(glide.position(after(start, 150)), before));
    assert!(!glide.is_moving(after(start, 300)));
}

#[test]
fn turns_toward_a_new_target_from_where_it_is() {
    let start = Instant::now();
    let mut glide = gliding(40.0, 10.0, start);
    let turning = after(start, 100);
    let there = glide.position(turning);

    glide.toward(30.0, turning);

    assert!(close(glide.position(turning), there));
    assert!(glide.is_moving(after(start, 399)));
    assert!(close(glide.position(after(start, 400)), 30.0));
}

#[test]
fn pins_along_its_own_curve_in_six_hundred_milliseconds() {
    let start = Instant::now();
    let mut pin = Glide::at_rest(-30.0, PIN);

    pin.toward(0.0, start);

    assert!(close(pin.position(start), -30.0));
    assert!(close(
        pin.position(after(start, 300)),
        -30.0 * (1.0 - 0.946_079_5)
    ));
    assert!(pin.is_moving(after(start, 599)));
    assert!(close(pin.position(after(start, 600)), 0.0));
    assert!(!pin.is_moving(after(start, 600)));
}

#[test]
fn settles_in_three_hundred_milliseconds_while_a_pin_takes_six_hundred() {
    let start = Instant::now();
    let mut settling = Glide::at_rest(1.0, SETTLE);
    let mut pinning = Glide::at_rest(1.0, PIN);

    settling.toward(0.0, start);
    pinning.toward(0.0, start);

    assert!(!settling.is_moving(after(start, 300)));
    assert!(pinning.is_moving(after(start, 300)));
}
