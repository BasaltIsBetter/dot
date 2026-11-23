use super::*;
use crate::relay::tests::Recording;

const PACE: Pace = Pace {
    hold: Duration::from_millis(100),
    pause: Duration::from_millis(30),
    wait: Duration::from_millis(70),
    held: 1_024,
};

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn waits_for_the_rest_of_the_hold_while_the_output_is_held() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    flow.take(b"a").unwrap();

    let at_the_start = timing.settle(start, &mut flow).unwrap();
    let before_the_end = timing.settle(after(start, 99), &mut flow).unwrap();
    let held = flow.is_holding();

    assert_eq!(at_the_start, Duration::from_millis(100));
    assert_eq!(before_the_end, Duration::from_millis(1));
    assert!(held);
    assert_eq!(recording.pieces, [""; 0]);
}

#[test]
fn lets_the_output_go_when_the_hold_is_over_then_waits_for_a_pause() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    flow.take(b"a").unwrap();

    let due = timing.settle(after(start, 100), &mut flow).unwrap();
    let held = flow.is_holding();

    assert_eq!(due, Duration::from_millis(30));
    assert!(!held);
    assert_eq!(recording.pieces, ["a"]);
    assert_eq!(recording.rests, 0);
}

#[test]
fn writes_what_was_held_back_after_a_pause_of_silence_and_not_before() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    timing.settle(after(start, 100), &mut flow).unwrap();

    let before = timing.settle(after(start, 129), &mut flow).unwrap();
    let at_the_pause = timing.settle(after(start, 130), &mut flow).unwrap();
    let after_it = timing.settle(after(start, 140), &mut flow).unwrap();

    assert_eq!(before, Duration::from_millis(1));
    assert_eq!(at_the_pause, Duration::from_millis(30));
    assert_eq!(after_it, Duration::from_millis(20));
    assert_eq!(recording.rests, 1);
    assert_eq!(written, b"<rest>");
}

#[test]
fn counts_the_silence_from_the_last_piece_heard() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    timing.settle(after(start, 100), &mut flow).unwrap();

    timing.heard(after(start, 125));
    let due = timing.settle(after(start, 130), &mut flow).unwrap();

    assert_eq!(due, Duration::from_millis(25));
    assert_eq!(recording.rests, 0);
}

#[test]
fn does_not_wait_for_a_hold_the_output_already_left() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 2);
    let mut timing = Timing::starting(PACE, start);
    flow.take(b"more than two").unwrap();
    timing.heard(after(start, 10));

    let due = timing.settle(after(start, 20), &mut flow).unwrap();

    assert_eq!(due, Duration::from_millis(20));
}

#[test]
fn writes_what_was_held_back_when_it_has_waited_long_enough_without_a_silence() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    timing.settle(after(start, 100), &mut flow).unwrap();
    for heard in [120, 140, 160] {
        timing.heard(after(start, heard));
    }

    let before = timing.settle(after(start, 169), &mut flow).unwrap();
    let at_the_wait = timing.settle(after(start, 170), &mut flow).unwrap();
    timing.heard(after(start, 175));
    let after_it = timing.settle(after(start, 180), &mut flow).unwrap();

    assert_eq!(before, Duration::from_millis(1));
    assert_eq!(at_the_wait, Duration::from_millis(30));
    assert_eq!(after_it, Duration::from_millis(25));
    assert_eq!(recording.rests, 1);
}

#[test]
fn counts_the_wait_again_from_each_time_it_wrote_what_was_held_back() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(PACE, start);
    timing.settle(after(start, 100), &mut flow).unwrap();
    timing.settle(after(start, 130), &mut flow).unwrap();
    for heard in [150, 170, 190] {
        timing.heard(after(start, heard));
    }

    let before = timing.settle(after(start, 199), &mut flow).unwrap();
    let at_the_wait = timing.settle(after(start, 200), &mut flow).unwrap();

    assert_eq!(before, Duration::from_millis(1));
    assert_eq!(at_the_wait, Duration::from_millis(30));
    assert_eq!(recording.rests, 2);
}

#[test]
fn counts_the_wait_from_the_start_for_an_output_that_left_the_hold_by_its_size() {
    let start = Instant::now();
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 2);
    let mut timing = Timing::starting(PACE, start);
    flow.take(b"more than two").unwrap();
    for heard in [20, 40, 60] {
        timing.heard(after(start, heard));
    }

    let before = timing.settle(after(start, 69), &mut flow).unwrap();
    let at_the_wait = timing.settle(after(start, 70), &mut flow).unwrap();

    assert_eq!(before, Duration::from_millis(1));
    assert_eq!(at_the_wait, Duration::from_millis(30));
    assert_eq!(recording.rests, 1);
}

#[test]
fn waits_no_longer_than_the_shorter_of_the_pause_and_the_wait() {
    let start = Instant::now();
    let hurried = Pace {
        wait: Duration::from_millis(10),
        ..PACE
    };
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    let mut timing = Timing::starting(hurried, start);
    timing.settle(after(start, 100), &mut flow).unwrap();

    let before = timing.settle(after(start, 105), &mut flow).unwrap();
    let at_the_wait = timing.settle(after(start, 110), &mut flow).unwrap();

    assert_eq!(before, Duration::from_millis(5));
    assert_eq!(at_the_wait, Duration::from_millis(10));
    assert_eq!(recording.rests, 1);
}
