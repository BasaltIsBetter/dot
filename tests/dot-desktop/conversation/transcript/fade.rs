use proptest::prelude::*;

use super::*;

fn after(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

#[test]
fn arrives_fully_orange() {
    assert_eq!(heat(Duration::ZERO), u8::MAX);
}

#[test]
fn cools_along_the_css_ease_out_curve() {
    assert_eq!(heat(after(100)), 221);
    assert_eq!(heat(after(300)), 159);
    assert_eq!(heat(after(600)), 80);
    assert_eq!(heat(after(900)), 24);
    assert_eq!(heat(after(1_100)), 3);
}

#[test]
fn settles_to_its_ink_after_one_point_two_seconds() {
    assert_eq!(heat(after(1_200)), 0);
    assert_eq!(heat(after(60_000)), 0);
}

#[test]
fn stays_fresh_until_it_has_settled() {
    assert!(is_fresh(Duration::ZERO));
    assert!(is_fresh(after(1_199)));
    assert!(!is_fresh(after(1_200)));
}

proptest! {
    #[test]
    fn only_ever_cools(earlier in 0_u64..2_000, later_by in 0_u64..2_000) {
        prop_assert!(heat(after(earlier)) >= heat(after(earlier + later_by)));
    }
}
