use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

fn typing() -> Pending {
    let mut pending = Pending::default();
    pending.gesture(Setting::ApiKey, &Gesture::Enter);
    pending
}

#[test]
fn leaves_choices_to_the_screen() {
    let mut pending = Pending::default();

    assert_eq!(pending.gesture(Setting::Model, &Gesture::Enter), None);
    assert_eq!(pending.gesture(Setting::Model, &Gesture::Left), None);
    assert_eq!(pending.waiting(), Waiting::Nothing);
}

#[test]
fn acts_on_the_folder_and_the_account_with_enter() {
    let mut pending = Pending::default();

    assert_eq!(
        pending.gesture(Setting::Folder, &Gesture::Enter),
        Some(Outcome::Act(Setting::Folder))
    );
    assert_eq!(
        pending.gesture(Setting::Account, &Gesture::Enter),
        Some(Outcome::Act(Setting::Account))
    );
}

#[test]
fn ignores_the_side_arrows_on_an_action_and_leaves_the_rest_to_the_screen() {
    let mut pending = Pending::default();

    assert_eq!(
        pending.gesture(Setting::Folder, &Gesture::Left),
        Some(Outcome::Nothing)
    );
    assert_eq!(
        pending.gesture(Setting::Folder, &Gesture::Right),
        Some(Outcome::Nothing)
    );
    assert_eq!(pending.gesture(Setting::Folder, &Gesture::Down), None);
}

#[test]
fn signs_out_only_on_a_second_enter() {
    let mut pending = Pending::default();

    assert_eq!(
        pending.gesture(Setting::SignOut, &Gesture::Enter),
        Some(Outcome::Nothing)
    );
    assert_eq!(pending.waiting(), Waiting::SignOut);
    assert_eq!(
        pending.gesture(Setting::SignOut, &Gesture::Enter),
        Some(Outcome::Act(Setting::SignOut))
    );
    assert_eq!(pending.waiting(), Waiting::Nothing);
}

#[test]
fn forgets_the_confirmation_at_any_other_gesture() {
    let mut pending = Pending::default();
    pending.gesture(Setting::SignOut, &Gesture::Enter);

    assert_eq!(pending.gesture(Setting::SignOut, &Gesture::Down), None);

    assert_eq!(pending.waiting(), Waiting::Nothing);
    assert_eq!(
        pending.gesture(Setting::SignOut, &Gesture::Enter),
        Some(Outcome::Nothing)
    );
}

#[test]
fn types_a_key_on_the_band_and_hands_it_over_with_enter() {
    let mut pending = typing();
    assert!(pending.is_typing_key());
    assert_eq!(pending.waiting(), Waiting::Key(0));

    pending.gesture(Setting::ApiKey, &Gesture::Type(format!("{KEY} \n")));
    assert_eq!(pending.waiting(), Waiting::Key(KEY.len()));
    let handed = pending.gesture(Setting::ApiKey, &Gesture::Enter);

    assert_eq!(handed, Some(Outcome::KeyTyped(ApiKey::parse(KEY).unwrap())));
    assert!(!pending.is_typing_key());
}

#[test]
fn erases_the_last_character_of_the_key() {
    let mut pending = typing();
    pending.gesture(Setting::ApiKey, &Gesture::Type("abc".to_owned()));

    pending.gesture(Setting::ApiKey, &Gesture::Erase);

    assert_eq!(pending.waiting(), Waiting::Key(2));
}

#[test]
fn refuses_a_malformed_key_and_keeps_it_for_correction() {
    let mut pending = typing();
    pending.gesture(Setting::ApiKey, &Gesture::Type("nope".to_owned()));

    let refused = pending.gesture(Setting::ApiKey, &Gesture::Enter);

    assert_eq!(refused, Some(Outcome::Refused(Text::SignInKeyMalformed)));
    assert_eq!(pending.waiting(), Waiting::Key(4));
}

#[test]
fn keeps_at_most_two_hundred_and_fifty_six_characters_of_a_key() {
    let mut pending = typing();

    pending.gesture(Setting::ApiKey, &Gesture::Type("a".repeat(300)));
    pending.gesture(Setting::ApiKey, &Gesture::Type("b".to_owned()));

    assert_eq!(pending.waiting(), Waiting::Key(256));
}

#[test]
fn any_other_gesture_drops_the_key_being_typed() {
    let mut pending = typing();
    pending.gesture(Setting::ApiKey, &Gesture::Type("abc".to_owned()));

    assert_eq!(
        pending.gesture(Setting::ApiKey, &Gesture::Escape),
        Some(Outcome::Nothing)
    );

    assert!(!pending.is_typing_key());
    assert_eq!(pending.waiting(), Waiting::Nothing);
}

#[test]
fn clears_what_waits() {
    let mut pending = typing();
    pending.clear();
    assert_eq!(pending.waiting(), Waiting::Nothing);

    pending.gesture(Setting::SignOut, &Gesture::Enter);
    pending.clear();
    assert_eq!(pending.waiting(), Waiting::Nothing);
}
