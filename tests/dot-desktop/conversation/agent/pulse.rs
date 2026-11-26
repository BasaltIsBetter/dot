use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn stays_full_until_it_beats() {
    let now = Instant::now();
    let mut pulse = Pulse::default();
    let idle = pulse.alpha(now);
    pulse.pace(false, now);

    assert_eq!(idle, OPAQUE);
    assert_eq!(pulse.alpha(after(now, 700)), OPAQUE);
    assert_eq!(pulse.next_change(now), None);
}

#[test]
fn dims_to_forty_five_percent_and_back_every_one_point_four_seconds() {
    let now = Instant::now();
    let mut pulse = Pulse::default();
    pulse.pace(true, now);
    pulse.pace(true, after(now, 500));

    let shades = [0, 350, 700, 1_050, 1_400, 2_100].map(|at| pulse.alpha(after(now, at)));

    assert_eq!(shades, [255, 185, 115, 185, 255, 115]);
}

#[test]
fn wakes_every_fifty_milliseconds_while_it_beats() {
    let now = Instant::now();
    let mut pulse = Pulse::default();
    pulse.pace(true, now);
    let beating = pulse.next_change(after(now, 30));

    pulse.pace(false, after(now, 60));

    assert_eq!(beating, Some(after(now, 80)));
    assert_eq!(pulse.next_change(after(now, 60)), None);
    assert_eq!(pulse.alpha(after(now, 700)), OPAQUE);
}

#[test]
fn starts_full_again_when_it_beats_anew() {
    let now = Instant::now();
    let mut pulse = Pulse::default();
    pulse.pace(true, now);
    pulse.pace(false, after(now, 700));

    pulse.pace(true, after(now, 1_000));

    assert_eq!(pulse.alpha(after(now, 1_000)), OPAQUE);
    assert_eq!(pulse.alpha(after(now, 1_700)), 115);
}
