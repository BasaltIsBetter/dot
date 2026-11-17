use winit::dpi::Size;

use super::*;

const TITLE: &str = "Dot";

#[test]
fn carries_the_translated_title() {
    assert_eq!(main_window_attributes(TITLE).title, TITLE);
}

#[test]
fn starts_hidden_until_its_background_is_black() {
    assert!(!main_window_attributes(TITLE).visible);
}

#[test]
fn opens_at_the_spaceship_size() {
    let attributes = main_window_attributes(TITLE);

    assert_eq!(attributes.inner_size, Some(Size::Logical(INITIAL_SIZE)));
    assert_eq!(attributes.min_inner_size, Some(Size::Logical(MINIMUM_SIZE)));
}

#[test]
fn asks_for_the_dark_appearance() {
    assert_eq!(
        main_window_attributes(TITLE).preferred_theme,
        Some(Theme::Dark)
    );
}
