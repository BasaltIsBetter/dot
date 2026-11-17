use super::*;

#[test]
fn names_each_standard_item_after_its_action() {
    let items = [
        (sel!(orderFrontStandardAboutPanel:), Text::MenuAbout),
        (sel!(hide:), Text::MenuHide),
        (sel!(hideOtherApplications:), Text::MenuHideOthers),
        (sel!(unhideAllApplications:), Text::MenuShowAll),
        (sel!(terminate:), Text::MenuQuit),
    ];

    for (action, text) in items {
        assert_eq!(menu_text(Some(action), false), Some(text));
    }
}

#[test]
fn names_the_item_that_opens_the_services() {
    assert_eq!(menu_text(None, true), Some(Text::MenuServices));
}

#[test]
fn leaves_unknown_items_alone() {
    assert_eq!(menu_text(Some(sel!(copy:)), false), None);
    assert_eq!(menu_text(None, false), None);
}
