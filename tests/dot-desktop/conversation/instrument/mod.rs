use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

const TEN_TOKENS: usize = 23;

fn replied(start: Instant, written: Option<u64>) -> Instrument {
    let mut instrument = Instrument::default();
    instrument.start(0, start);
    instrument.hear(TEN_TOKENS, after(start, 100));
    instrument.end(written, after(start, 1_100));
    instrument
}

#[test]
fn shows_nothing_before_the_first_prompt() {
    let instrument = Instrument::default();

    assert_eq!(instrument.reading(Instant::now()), None);
    assert_eq!(instrument.current(Instant::now()), None);
    assert_eq!(instrument.next_change(Instant::now()), None);
    assert!(!instrument.is_animating(Instant::now()));
}

#[test]
fn appears_in_a_hundred_and_twenty_milliseconds_and_reads_orange_while_claude_writes() {
    let start = Instant::now();
    let mut instrument = Instrument::default();

    instrument.start(0, start);
    instrument.hear(TEN_TOKENS, after(start, 50));

    let appearing = instrument.reading(after(start, 60)).unwrap();
    let shown = instrument.reading(after(start, 120)).unwrap();
    assert!(appearing.alpha > 0 && appearing.alpha < u8::MAX);
    assert_eq!(shown.alpha, u8::MAX);
    assert_eq!(shown.heat, u8::MAX);
    assert_eq!(shown.measures.unwrap().tokens, 10);
    assert!(instrument.is_animating(after(start, 60)));
    assert_eq!(
        instrument.next_change(after(start, 120)),
        Some(after(start, 200))
    );
}

#[test]
fn ends_with_the_tokens_claude_counted_and_cools_to_white_in_one_point_two_seconds() {
    let start = Instant::now();
    let instrument = replied(start, Some(40));

    let cooling = instrument.reading(after(start, 1_400)).unwrap();
    let cooled = instrument.reading(after(start, 2_300)).unwrap();

    assert_eq!(cooled.measures.unwrap().tokens, 40);
    assert!(
        cooling.heat > 0 && cooling.heat < u8::MAX,
        "{}",
        cooling.heat
    );
    assert_eq!(cooled.heat, 0);
    assert!(instrument.is_animating(after(start, 2_299)));
    assert!(!instrument.is_animating(after(start, 2_300)));
    assert_eq!(instrument.next_change(after(start, 2_300)), None);
}

#[test]
fn learns_how_many_tokens_a_character_is_worth_from_each_reply() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(46));

    instrument.start(1, after(start, 5_000));
    instrument.hear(TEN_TOKENS, after(start, 5_100));

    assert_eq!(instrument.current(after(start, 5_100)).unwrap().tokens, 46);
}

#[test]
fn a_reply_without_its_count_teaches_nothing() {
    let start = Instant::now();
    let mut instrument = replied(start, None);

    instrument.start(1, after(start, 5_000));
    instrument.hear(TEN_TOKENS, after(start, 5_100));

    assert_eq!(instrument.current(after(start, 5_100)).unwrap().tokens, 10);
}

#[test]
fn shows_the_measures_of_the_exchange_read_and_fades_them_in() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(40));
    instrument.start(1, after(start, 5_000));
    instrument.hear(TEN_TOKENS * 3, after(start, 5_100));

    instrument.follow(Shown::Exchange(Some(0)), after(start, 6_000));

    let switching = instrument.reading(after(start, 6_080)).unwrap();
    let switched = instrument.reading(after(start, 6_160)).unwrap();
    assert_eq!(switched.measures.unwrap().tokens, 40);
    assert_eq!(switched.heat, 0);
    assert!(switching.values_alpha < u8::MAX);
    assert_eq!(switched.values_alpha, u8::MAX);
    assert!(instrument.is_animating(after(start, 6_080)));
}

#[test]
fn the_exchange_being_replied_shows_its_live_measures_in_white() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(40));
    instrument.start(1, after(start, 5_000));
    instrument.hear(TEN_TOKENS * 3, after(start, 5_100));

    instrument.follow(Shown::Exchange(Some(1)), after(start, 5_200));

    let reading = instrument.reading(after(start, 5_200)).unwrap();
    assert_eq!(reading.measures.unwrap().tokens, 120);
    assert_eq!(reading.heat, 0);
}

#[test]
fn an_exchange_without_measures_shows_dashes() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(40));

    instrument.follow(Shown::Exchange(None), after(start, 3_000));
    let nothing = instrument.reading(after(start, 3_000)).unwrap();
    instrument.follow(Shown::Exchange(Some(7)), after(start, 3_100));
    let unknown = instrument.reading(after(start, 3_100)).unwrap();

    assert_eq!(nothing.measures, None);
    assert_eq!(unknown.measures, None);
}

#[test]
fn reviewing_before_any_prompt_shows_the_instrument_with_dashes() {
    let start = Instant::now();
    let mut instrument = Instrument::default();

    instrument.follow(Shown::Exchange(None), start);

    assert_eq!(
        instrument.reading(after(start, 200)).unwrap().measures,
        None
    );
}

#[test]
fn coming_back_shows_the_current_reply_again() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(40));
    instrument.follow(Shown::Exchange(None), after(start, 3_000));

    instrument.follow(Shown::Current, after(start, 3_100));
    let again = instrument.switched;
    instrument.follow(Shown::Current, after(start, 3_200));

    assert_eq!(
        instrument
            .reading(after(start, 3_500))
            .unwrap()
            .measures
            .unwrap()
            .tokens,
        40
    );
    assert_eq!(instrument.switched, again);
}

#[test]
fn ends_a_reply_once() {
    let start = Instant::now();
    let mut instrument = replied(start, Some(40));

    instrument.end(Some(99), after(start, 2_000));

    assert_eq!(instrument.past.len(), 1);
    assert_eq!(instrument.current(after(start, 2_000)).unwrap().tokens, 40);
}

#[test]
fn fades_in_along_the_settling_curve() {
    let start = Instant::now();

    assert_eq!(faded_in(start, APPEARING, start), 0);
    assert_eq!(faded_in(start, APPEARING, after(start, 120)), u8::MAX);
    assert_eq!(
        faded_in(start, APPEARING, after(start, 60)),
        opacity(SETTLE.eased(Duration::from_millis(150)).unwrap())
    );
}
