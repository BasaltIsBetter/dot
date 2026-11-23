use proptest::prelude::*;

use super::*;
use crate::status_bar::view::tests::ShortTranslator;

#[test]
fn counts_tokens_in_thousands_below_a_million() {
    assert_eq!(token_count(0), "0k");
    assert_eq!(token_count(999), "0k");
    assert_eq!(token_count(1_000), "1k");
    assert_eq!(token_count(147_999), "147k");
    assert_eq!(token_count(999_999), "999k");
}

#[test]
fn counts_tokens_in_tenths_of_a_million_from_a_million() {
    assert_eq!(token_count(1_000_000), "1.0M");
    assert_eq!(token_count(1_049_999), "1.0M");
    assert_eq!(token_count(1_050_000), "1.1M");
    assert_eq!(token_count(2_500_000), "2.5M");
    assert_eq!(token_count(u64::MAX), "18446744073709.6M");
}

#[test]
fn writes_dollars_to_the_nearest_cent() {
    let dollars =
        |micro_dollars| dollars(Cost::from_micro_dollars(micro_dollars), &ShortTranslator);

    assert_eq!(dollars(0), "0.00");
    assert_eq!(dollars(4_999), "0.00");
    assert_eq!(dollars(5_000), "0.01");
    assert_eq!(dollars(2_400_000), "2.40");
    assert_eq!(dollars(123_456_789), "123.46");
}

#[test]
fn shares_a_part_of_a_whole_rounded_down() {
    assert_eq!(share(150_000, 1_000_000, 100), 15);
    assert_eq!(share(999, 1_000, 100), 99);
    assert_eq!(share(147_000, 1_000_000, 1_000), 147);
    assert_eq!(share(0, 1_000_000, 100), 0);
    assert_eq!(share(1_000_000, 1_000_000, 100), 100);
}

#[test]
fn never_shares_more_than_the_whole() {
    assert_eq!(share(2_000_000, 1_000_000, 100), 100);
    assert_eq!(share(u64::MAX, 1, 1_000), 1_000);
}

#[test]
fn shares_nothing_of_an_empty_whole() {
    assert_eq!(share(5, 0, 100), 0);
}

proptest! {
    #[test]
    fn counts_any_number_of_tokens_in_a_few_cells(tokens in any::<u64>()) {
        let text = token_count(tokens);

        prop_assert!(text.len() <= 17);
        prop_assert!(text.ends_with('k') || text.ends_with('M'));
    }

    #[test]
    fn writes_any_cost_with_two_decimals(micro_dollars in any::<u64>()) {
        let text = dollars(Cost::from_micro_dollars(micro_dollars), &ShortTranslator);
        let (_, cents) = text.split_once('.').unwrap();

        prop_assert_eq!(cents.len(), 2);
    }

    #[test]
    fn never_shares_past_the_scale(part in any::<u64>(), whole in any::<u64>(), scale in 1_u16..2_000) {
        prop_assert!(share(part, whole, scale) <= scale);
    }
}
