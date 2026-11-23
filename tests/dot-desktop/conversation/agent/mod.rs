use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn fades_in_from_a_little_smaller_over_450_milliseconds() {
    let start = Instant::now();
    let view = AgentView::open("toolu_1".to_owned(), start);

    let opening = view.fading(start).unwrap();
    let halfway = view.fading(after(start, 225)).unwrap();

    assert_eq!(opening.alpha, 0);
    assert!((opening.scale - 0.97).abs() < 1e-9);
    assert!(halfway.alpha > 128 && halfway.scale > 0.99 && halfway.scale < 1.0);
    assert_eq!(view.fading(after(start, 450)), None);
}

#[test]
fn the_conversation_comes_back_from_a_little_larger() {
    let start = Instant::now();

    let back = fading_back(start, start).unwrap();
    let halfway = fading_back(start, after(start, 225)).unwrap();

    assert!((back.scale - 1.03).abs() < 1e-9);
    assert!(halfway.scale > 1.0 && halfway.scale < 1.01);
    assert_eq!(fading_back(start, after(start, 450)), None);
}

#[test]
fn scrolls_within_its_rows() {
    let mut view = AgentView::open("toolu_1".to_owned(), Instant::now());

    view.scroll_by(3.0, 10);
    let up = view.scrolled();
    view.scroll_by(30.0, 10);
    let top = view.scrolled();
    view.scroll_by(-50.0, 10);

    assert!((up - 3.0).abs() < f64::EPSILON);
    assert!((top - 10.0).abs() < f64::EPSILON);
    assert!(view.scrolled().abs() < f64::EPSILON);
}
