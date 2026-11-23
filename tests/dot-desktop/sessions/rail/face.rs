use crate::sessions::State;
use crate::sessions::rail::Hit;
use crate::sessions::rail::tests::{card, centre_of, rail_of, texts, two_cards};

#[test]
fn offers_to_close_only_the_card_under_the_pointer_and_never_the_last_session() {
    let unhovered = rail_of(&two_cards(), None);
    let over_the_second = centre_of(&unhovered, Hit::Switch(1));

    let hovered = rail_of(&two_cards(), Some(over_the_second));
    let alone = rail_of(
        &two_cards()[..1],
        Some(centre_of(&unhovered, Hit::Switch(0))),
    );

    assert!(
        !unhovered
            .hits
            .iter()
            .any(|(_, hit)| matches!(hit, Hit::Close(_)))
    );
    assert!(hovered.hits.iter().any(|(_, hit)| *hit == Hit::Close(1)));
    assert!(texts(&hovered).contains(&"×".to_owned()));
    assert!(
        !alone
            .hits
            .iter()
            .any(|(_, hit)| matches!(hit, Hit::Close(_)))
    );
}

#[test]
fn writes_each_name_with_its_glyph_and_the_rate_of_its_last_reply() {
    let rail = rail_of(&two_cards(), None);

    let texts = texts(&rail);
    assert!(texts.contains(&"Dot".to_owned()));
    assert!(texts.contains(&"◉".to_owned()));
    assert!(texts.contains(&"●".to_owned()));
    assert!(texts.contains(&"58 unit-tokens-per-second".to_owned()));
    assert!(texts.contains(&"––– unit-tokens-per-second".to_owned()));
}

#[test]
fn a_card_without_a_reply_has_no_foot() {
    let cards = [card("Dot", true, State::Idle, None)];

    let rail = rail_of(&cards, None);

    assert_eq!(texts(&rail), ["Dot", "◉"]);
}
