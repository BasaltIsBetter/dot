use std::time::Duration;

use super::*;
use crate::app::tests::IdTranslator;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.text.as_str()).collect())
        .collect()
}

#[test]
fn shows_at_once_at_rest_when_nothing_happened_yet() {
    let start = Instant::now();
    let mut help = Help::default();

    help.pace(true, true, start);

    assert!((help.alpha(start) - 1.0).abs() < f64::EPSILON);
    assert!(!help.is_fading(start));
    assert_eq!(help.wakes_at(start), None);
}

#[test]
fn hides_while_something_happens() {
    let start = Instant::now();
    let mut help = Help::default();

    help.pace(false, true, start);

    assert!(help.alpha(start).abs() < f64::EPSILON);
    assert_eq!(help.lines(&IdTranslator, start), Vec::new());
}

#[test]
fn comes_back_after_five_quiet_seconds_and_fades_in() {
    let start = Instant::now();
    let mut help = Help::default();
    help.pace(true, true, start);

    help.stir(after(start, 100));
    help.pace(true, true, after(start, 100));
    let wake = help.wakes_at(after(start, 100));
    help.pace(true, true, after(start, 5_099));
    let still_quiet = help.alpha(after(start, 5_099));
    help.pace(true, true, after(start, 5_100));

    assert_eq!(wake, Some(after(start, 5_100)));
    assert!(help.alpha(after(start, 250)) < 1.0);
    assert!(still_quiet.abs() < f64::EPSILON);
    assert!(help.is_fading(after(start, 5_200)));
    let halfway = help.alpha(after(start, 5_250));
    assert!((halfway - 0.946).abs() < 0.001, "{halfway}");
    assert!((help.alpha(after(start, 5_400)) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn wakes_nobody_in_the_background() {
    let start = Instant::now();
    let mut help = Help::default();
    help.stir(start);

    help.pace(true, false, start);

    assert_eq!(help.wakes_at(start), None);
}

#[test]
fn lines_up_each_key_and_what_it_does() {
    let start = Instant::now();
    let mut help = Help::default();
    help.pace(true, true, start);

    let lines = help.lines(&IdTranslator, start);

    assert_eq!(
        texts(&lines),
        vec![
            "help-send-key      help-send",
            "help-new-line-key  help-new-line",
            "help-pause-key     help-pause",
            "help-stop-key      help-stop",
            "help-review-key    help-review",
            "help-switch-key    help-switch",
            "help-settings-key  help-settings",
        ]
    );
    assert_eq!(lines[0].spans[0].ink, Ink::Full);
    assert_eq!(lines[0].spans[1].ink, Ink::Secondary);
}

#[test]
fn wakes_nobody_once_the_quiet_seconds_are_over() {
    let start = Instant::now();
    let mut help = Help::default();
    help.pace(true, true, start);
    help.stir(start);
    help.pace(true, true, start);

    assert_eq!(help.wakes_at(after(start, 5_000)), None);
    assert_eq!(
        help.wakes_at(after(start, 4_999)),
        Some(after(start, 5_000))
    );
}
