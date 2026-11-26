use std::time::Instant;

use dot_api::Text;

use super::*;
use crate::app::tests::IdTranslator;
use crate::settings::Waiting;
use crate::settings::reel::{Reel, place_of};
use crate::settings::tests::test_settings;
use crate::settings::view::tests::{frame, layout};
use crate::settings::view::{Look, entries};

fn drawn_on(setting: Setting, waiting: Waiting, actions: &Actions) -> Drawn {
    let layout = layout();
    let settings = test_settings();
    let now = Instant::now();
    let mut reel = Reel::new();
    reel.focus_on(place_of(setting), now);
    let look = Look {
        settings: &settings,
        defaults: &settings,
        translator: &IdTranslator,
        hover: None,
        changed: None,
        actions,
        waiting,
    };
    entries(
        &reel,
        &look,
        (&layout, &frame(&layout)),
        now + std::time::Duration::from_secs(1),
    )
}

fn actions() -> Actions {
    Actions {
        folder: "/Users/me/Desktop/Dot".to_owned(),
        account: "Compte Claude".to_owned(),
        key: "•••• abcd".to_owned(),
    }
}

fn shown(drawn: &Drawn, text: &str) -> Option<Ink> {
    drawn
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .find(|span| span.text == text)
        .map(|span| span.ink)
}

#[test]
fn shows_the_folder_the_account_and_the_key_at_the_right_of_their_rows() {
    let drawn = drawn_on(Setting::Folder, Waiting::Nothing, &actions());
    let connection = drawn_on(Setting::Account, Waiting::Nothing, &actions());

    assert_eq!(shown(&drawn, "/Users/me/Desktop/Dot"), Some(Ink::Full));
    assert!(shown(&connection, "Compte Claude").is_some());
    assert!(shown(&connection, "•••• abcd").is_some());
    let folder = drawn
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Choose(Setting::Folder, 0))
        .unwrap()
        .0;
    let frame = frame(&layout());
    assert!((folder[0] + folder[2] - frame.right()).abs() < 0.01);
}

#[test]
fn asks_for_a_second_enter_in_the_live_colour_before_signing_out() {
    let resting = drawn_on(Setting::SignOut, Waiting::Nothing, &actions());
    let waiting = drawn_on(Setting::SignOut, Waiting::SignOut, &actions());

    let confirm = Text::SignOutConfirm.id();
    assert_eq!(shown(&resting, confirm), None);
    assert_eq!(shown(&waiting, confirm), Some(Ink::Live));
}

#[test]
fn follows_the_focused_action_with_an_enter_arrow_like_the_prototype() {
    let drawn = drawn_on(Setting::Folder, Waiting::Nothing, &actions());

    assert_eq!(shown(&drawn, "  ↵"), Some(Ink::Receding));
    let arrows = drawn
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .filter(|span| span.text.contains('↵'))
        .count();
    assert_eq!(arrows, 1);
}

#[test]
fn shows_only_the_arrow_on_sign_out_when_it_has_the_focus() {
    let resting = drawn_on(Setting::SignOut, Waiting::Nothing, &actions());
    let elsewhere = drawn_on(Setting::Folder, Waiting::Nothing, &actions());

    assert_eq!(shown(&resting, "↵"), Some(Ink::Receding));
    assert!(
        resting
            .hits
            .iter()
            .any(|(_, hit)| *hit == Hit::Choose(Setting::SignOut, 0))
    );
    assert!(
        !elsewhere
            .hits
            .iter()
            .any(|(_, hit)| *hit == Hit::Choose(Setting::SignOut, 0))
    );
}

#[test]
fn hides_the_arrow_while_something_live_is_shown() {
    let confirming = drawn_on(Setting::SignOut, Waiting::SignOut, &actions());
    let typing = drawn_on(Setting::ApiKey, Waiting::Key(2), &actions());

    for drawn in [confirming, typing] {
        assert!(
            drawn
                .panes
                .iter()
                .flat_map(|pane| &pane.lines)
                .flat_map(|placed| &placed.line.spans)
                .all(|span| !span.text.contains('↵'))
        );
    }
}

#[test]
fn shows_the_key_being_typed_as_live_dots_and_a_caret() {
    let drawn = drawn_on(Setting::ApiKey, Waiting::Key(5), &actions());

    assert_eq!(shown(&drawn, "•••••_"), Some(Ink::Live));
    assert_eq!(shown(&drawn, "•••• abcd"), None);
}

#[test]
fn keeps_the_end_of_a_value_too_long_for_its_row() {
    let long = Actions {
        folder: format!("/{}", "a".repeat(400)),
        ..actions()
    };

    let drawn = drawn_on(Setting::Folder, Waiting::Nothing, &long);

    let folder = drawn
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .find(|span| span.text.starts_with('…'))
        .unwrap();
    assert!(folder.text.ends_with("aaa"));
    let advance = layout().advance_of(TextSize::Regular);
    let room = whole(frame(&layout()).width / advance);
    let label = Text::SettingFolder.id().chars().count();
    assert!(folder.text.chars().count() <= room - label - 2);
}

#[test]
fn keeps_a_value_whole_when_it_fits_and_nothing_when_there_is_no_room() {
    assert_eq!(ending("abc", 3), "abc");
    assert_eq!(ending("abcd", 3), "…cd");
    assert_eq!(ending("abcd", 1), "…");
    assert_eq!(ending("abcd", 0), "");
    assert_eq!(whole(-3.0), 0);
    assert_eq!(whole(7.9), 7);
}
