use proptest::prelude::*;

use super::*;
use crate::pricing::price_of;

const OPUS: &str = "claude-opus-5-5";
const WHOLE_WINDOW: u64 = 1_000_000;

fn hour_long(read: u64) -> Request {
    Request {
        read,
        written: 1_000,
        written_for_an_hour: 1_000,
    }
}

fn after(requests: impl IntoIterator<Item = Request>) -> Ledger {
    let mut ledger = Ledger::NOTHING_YET;
    for request in requests {
        ledger.request(request);
    }
    ledger
}

fn base_then(base: u64, context: u64, times: usize) -> Ledger {
    after(std::iter::once(hour_long(base)).chain(std::iter::repeat_n(hour_long(context), times)))
}

fn window(ledger: &Ledger) -> Option<u64> {
    ledger.window(&price_of(OPUS), WHOLE_WINDOW)
}

#[test]
fn hands_no_window_over_before_ten_requests() {
    assert_eq!(window(&base_then(100_000, 400_000, 8)), None);
    assert!(window(&base_then(100_000, 400_000, 9)).is_some());
}

#[test]
fn a_conversation_that_stays_small_gets_the_lowest_window_that_cannot_loop() {
    let small = base_then(50_000, 60_000, 9);

    assert_eq!(window(&small), Some((50_000 + 100_000) * 5 / 4 + 33_000));
}

#[test]
fn keeps_the_window_above_the_context_until_the_surplus_paid_reaches_the_cost_of_a_summary() {
    let not_yet = base_then(100_000, 400_000, 12);
    let due = base_then(100_000, 400_000, 22);

    assert!(
        window(&not_yet).unwrap() > 400_000,
        "{:?}",
        window(&not_yet)
    );
    let now = window(&due).unwrap();
    assert!((283_000..=400_000).contains(&now), "{now}");
}

#[test]
fn places_the_window_where_the_surplus_would_have_paid_for_a_summary() {
    let ledger = base_then(100_000, 400_000, 12);

    let left_after_a_summary = 100_000 + 26_000 + 20_000;
    let paid = 12 * (400_000 - left_after_a_summary);
    let rewritten = left_after_a_summary * 780 / 1_000;
    let cost = rewritten * 8_000 / 200 + 6_000 * 20_000 / 200 + 400_000;
    let growth = 300_000 / 13;
    let over: u64 = 400_000 - left_after_a_summary;
    let climb = (over * over + 2 * growth * (cost - paid)).isqrt();
    assert_eq!((cost, paid, climb), (5_555_200, 3_048_000, 424_533));
    assert_eq!(window(&ledger), Some(left_after_a_summary + climb + 33_000));
}

#[test]
fn the_window_closes_in_as_the_surplus_adds_up() {
    let early = window(&base_then(100_000, 400_000, 10)).unwrap();
    let later = window(&base_then(100_000, 400_000, 16)).unwrap();

    assert!(later < early, "{later} {early}");
}

#[test]
fn a_summary_starts_the_count_again_from_what_it_left() {
    let mut ledger = base_then(100_000, 400_000, 22);

    ledger.summarized();
    ledger.request(Request {
        read: 150_000,
        written: 120_000,
        written_for_an_hour: 120_000,
    });

    let after_it = window(&ledger).unwrap();
    assert!(after_it > 150_000, "{after_it}");
    assert!(after_it >= 150_000 * 5 / 4 + 33_000, "{after_it}");
}

fn summarized_then_grown(written: u64) -> Ledger {
    let mut ledger = base_then(100_000, 400_000, 22);
    ledger.summarized();
    ledger.request(Request {
        read: 150_000,
        written,
        written_for_an_hour: written,
    });
    for _ in 0..6 {
        ledger.request(hour_long(500_000));
    }
    ledger
}

#[test]
fn a_summary_that_had_little_rewritten_makes_the_next_one_come_sooner() {
    let cheap = window(&summarized_then_grown(10_000)).unwrap();
    let dear = window(&summarized_then_grown(140_000)).unwrap();

    assert!(cheap < dear, "{cheap} {dear}");
}

#[test]
fn a_five_minute_cache_makes_a_summary_cheaper_so_it_comes_sooner() {
    let short_lived = Request {
        read: 400_000,
        written: 1_000,
        written_for_an_hour: 0,
    };
    let five_minutes =
        after(std::iter::once(hour_long(100_000)).chain(std::iter::repeat_n(short_lived, 12)));
    let an_hour = base_then(100_000, 400_000, 12);

    assert!(window(&five_minutes).unwrap() < window(&an_hour).unwrap());
}

#[test]
fn a_request_that_wrote_nothing_does_not_change_how_long_the_cache_lasts() {
    let nothing_written = Request {
        read: 400_000,
        written: 0,
        written_for_an_hour: 0,
    };
    let mut ledger = base_then(100_000, 400_000, 11);
    let before = ledger;
    ledger.request(nothing_written);
    let mut hour_long_all_along = before;
    hour_long_all_along.request(hour_long(400_000));

    assert_eq!(window(&ledger), window(&hour_long_all_along));
}

#[test]
fn never_goes_below_claude_code_s_minimum_nor_above_the_model_s_window() {
    let mut ledger = base_then(10_000, 20_000, 12);
    ledger.summarized();
    ledger.request(hour_long(30_000));

    assert_eq!(window(&ledger), Some(100_000));
    let filling = base_then(100_000, 400_000, 10);
    assert_eq!(filling.window(&price_of(OPUS), 200_000), Some(200_000));
}

#[test]
fn passes_over_a_request_claude_never_read() {
    let mut ledger = base_then(100_000, 400_000, 8);
    let before = ledger;

    ledger.request(hour_long(0));

    assert_eq!(ledger, before);
    assert_eq!(window(&ledger), None);
}

#[test]
fn takes_the_first_request_claude_read_as_the_base() {
    let unread_first = after([hour_long(0), hour_long(50_000)]);

    assert_eq!(unread_first, after([hour_long(50_000)]));
}

proptest! {
    #[test]
    fn any_conversation_gets_a_window_claude_code_accepts_or_none(
        requests in proptest::collection::vec((0..3_000_000_u64, 0..3_000_000_u64, any::<bool>(), any::<bool>()), 0..60),
        largest in 150_000..2_000_000_u64,
    ) {
        let mut ledger = Ledger::NOTHING_YET;
        let mut read_ones = 0;
        for (read, written, hour, summary) in requests {
            if summary {
                ledger.summarized();
            }
            ledger.request(Request { read, written, written_for_an_hour: if hour { written } else { 0 } });
            read_ones += u64::from(read > 0);

            let window = ledger.window(&price_of(OPUS), largest);
            prop_assert_eq!(window.is_some(), read_ones >= 10);
            if let Some(window) = window {
                prop_assert!((100_000..=largest).contains(&window), "{}", window);
            }
        }
    }
}
fn is_due(ledger: &Ledger) -> bool {
    ledger.summary_is_due(&price_of(OPUS))
}

#[test]
fn a_summary_is_due_once_the_surplus_has_paid_for_it() {
    assert!(!is_due(&base_then(100_000, 400_000, 12)));
    assert!(is_due(&base_then(100_000, 400_000, 22)));
}

#[test]
fn a_summary_is_never_due_in_the_first_ten_requests() {
    assert!(!is_due(&base_then(100_000, 2_000_000, 8)));
    assert!(is_due(&base_then(100_000, 2_000_000, 9)));
}

#[test]
fn a_summary_is_not_due_when_the_context_is_too_close_to_what_it_would_leave() {
    let mut shrunk = base_then(100_000, 400_000, 22);
    shrunk.request(hour_long(282_999));
    let mut roomy = base_then(100_000, 400_000, 22);
    roomy.request(hour_long(283_000));

    assert!(!is_due(&shrunk));
    assert!(is_due(&roomy));
}

#[test]
fn nothing_is_decided_between_a_summary_and_the_request_that_shows_what_it_left() {
    let mut ledger = base_then(100_000, 400_000, 22);

    ledger.summarized();

    assert!(!is_due(&ledger));
    assert_eq!(window(&ledger), None);
}
