use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use super::*;
use crate::app::tests::{
    RETINA, RETINA_SIZE, RecordingCanvas, TestApp, act, app_with, canvas_of, connected_with_status,
    named, pane_texts, shown_row, signing_in, typed,
};
use crate::event::Wheel;

const API_KEY_ROW: usize = 4;
const POINTER: PhysicalPosition<f64> = PhysicalPosition::new(300.4, 200.6);

fn over_row(row: usize) -> PhysicalPosition<f64> {
    let layout = Layout::new(RETINA_SIZE, RETINA);
    PhysicalPosition::new(
        f64::from(layout.x(0)) + 1.0,
        f64::from(layout.line_top(row)) + 1.0,
    )
}

fn pointed() -> TestApp {
    let mut app = app_with(RecordingCanvas::default());
    act(&mut app, WindowAction::Point(POINTER));
    app
}

fn sent_prompts(count: usize) -> TestApp {
    let mut app = connected_with_status();
    for number in 0..count {
        act(&mut app, typed(&number.to_string()));
        act(&mut app, named(NamedKey::Enter));
    }
    app
}

#[test]
fn a_key_moves_the_selection_on_screen() {
    let mut app = signing_in();

    act(&mut app, named(NamedKey::ArrowDown));

    assert!(shown_row(&app, API_KEY_ROW).starts_with("› sign-in-api-key"));
}

#[test]
fn composed_text_goes_to_the_key_field_while_signing_in() {
    let mut app = signing_in();
    act(&mut app, named(NamedKey::ArrowDown));
    act(&mut app, named(NamedKey::Enter));

    act(&mut app, WindowAction::Commit("sk".to_owned()));

    assert_eq!(shown_row(&app, 3), "› ••_");
}

#[test]
fn hovering_a_way_selects_it() {
    let mut app = signing_in();

    act(&mut app, WindowAction::Point(over_row(API_KEY_ROW)));

    assert!(shown_row(&app, API_KEY_ROW).starts_with("› "));
}

#[test]
fn clicking_a_way_chooses_it() {
    let mut app = signing_in();
    act(&mut app, WindowAction::Point(over_row(API_KEY_ROW)));

    act(&mut app, WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-key-prompt");
}

#[test]
fn clicking_outside_the_ways_does_nothing() {
    let mut app = signing_in();
    act(&mut app, WindowAction::Point(over_row(0)));

    act(&mut app, WindowAction::Press);

    assert_eq!(shown_row(&app, 1), "sign-in-intro");
}

#[test]
fn typing_once_connected_writes_in_the_prompter() {
    let mut app = connected_with_status();

    act(&mut app, typed("h"));
    act(&mut app, typed("i"));
    act(&mut app, WindowAction::Commit("é".to_owned()));

    assert_eq!(pane_texts(&app, 1), vec!["› hié", "_"]);
}

#[test]
fn shows_a_composition_in_the_prompter() {
    let mut app = connected_with_status();

    act(&mut app, WindowAction::Compose("^".to_owned()));

    assert_eq!(pane_texts(&app, 1), vec!["› ", "^"]);
}

#[test]
fn ignores_a_composition_while_signing_in() {
    let mut app = app_with(RecordingCanvas::default());
    act(&mut app, WindowAction::Compose("^".to_owned()));
    app.sign_in.report(crate::sign_in::Report::Checked(Ok(Some(
        dot_api::Connection::ApiKey,
    ))));

    act(&mut app, WindowAction::Paint);

    assert_eq!(pane_texts(&app, 1), vec!["› ", "_", "prompter-placeholder"]);
}

#[test]
fn return_sends_the_prompt_above() {
    let mut app = connected_with_status();
    act(&mut app, typed("hello"));

    act(&mut app, named(NamedKey::Enter));

    assert_eq!(pane_texts(&app, 0), vec!["› hello"]);
    assert_eq!(pane_texts(&app, 1), vec!["› ", "_", "prompter-placeholder"]);
}

#[test]
fn the_wheel_scrolls_the_transcript_under_the_pointer() {
    let mut app = sent_prompts(60);
    act(
        &mut app,
        WindowAction::Point(PhysicalPosition::new(1_000.0, 400.0)),
    );
    let before = pane_texts(&app, 0);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(5.0)));

    assert_ne!(pane_texts(&app, 0), before);
}

#[test]
fn the_wheel_does_nothing_without_a_pointer_or_a_connection() {
    let mut app = sent_prompts(60);
    let before = pane_texts(&app, 0);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(5.0)));
    let unpointed = pane_texts(&app, 0);
    let mut signing = signing_in();
    act(
        &mut signing,
        WindowAction::Point(PhysicalPosition::new(1_000.0, 400.0)),
    );
    act(&mut signing, WindowAction::Scroll(Wheel::Lines(5.0)));

    assert_eq!(unpointed, before);
    assert_eq!(canvas_of(&signing).shown.panes, Vec::new());
}

#[test]
fn clicking_in_the_prompter_moves_the_caret() {
    let layout = Layout::new(RETINA_SIZE, RETINA);
    let mut app = connected_with_status();
    act(&mut app, typed("hello"));
    let caret_row_top = canvas_of(&app).shown.panes[1].lines[1].top;

    act(
        &mut app,
        WindowAction::Point(PhysicalPosition::new(
            f64::from(layout.x(3)) + 1.0,
            f64::from(caret_row_top) + 1.0,
        )),
    );
    act(&mut app, WindowAction::Press);

    assert_eq!(canvas_of(&app).shown.panes[1].lines[1].line.indent, 3);
}

#[test]
fn draws_the_reticle_where_the_pointer_is() {
    let app = pointed();

    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, RETINA)));
}

#[test]
fn hides_the_reticle_when_the_pointer_leaves_the_window() {
    let mut app = pointed();

    act(&mut app, WindowAction::Leave);

    assert_eq!(canvas_of(&app).reticle, None);
}

#[test]
fn hides_the_reticle_while_the_window_is_in_the_background() {
    let mut app = pointed();

    act(&mut app, WindowAction::Blur);
    let blurred = canvas_of(&app).reticle;
    act(&mut app, WindowAction::Focus);

    assert_eq!(blurred, None);
    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, RETINA)));
}

#[test]
fn resizes_the_ring_with_the_screen_scale() {
    let mut app = pointed();

    act(&mut app, WindowAction::Rescale(1.0));

    assert_eq!(canvas_of(&app).reticle, Some(Reticle::at(POINTER, 1.0)));
}
