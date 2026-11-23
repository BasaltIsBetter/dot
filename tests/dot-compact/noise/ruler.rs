use proptest::prelude::*;

use super::*;

#[test]
fn keeps_three_characters_of_a_long_ruler() {
    assert_eq!(
        shorten("a ━━━━━━━━━━ b\n┌──────┬────┐\n═════\n").as_deref(),
        Some("a ━━━ b\n┌───┬───┐\n═══\n")
    );
}

#[test]
fn leaves_short_rulers_and_plain_dashes_alone() {
    assert_eq!(shorten("├── name\n━━━\n"), None);
    assert_eq!(shorten("----------\n==========\n"), None);
    assert_eq!(shorten("no ruler at all"), None);
    assert_eq!(shorten(""), None);
}

#[test]
fn counts_each_ruler_on_its_own() {
    assert_eq!(shorten("───━━━═══────").as_deref(), Some("───━━━═══───"));
    assert_eq!(shorten("aaaaaa ────").as_deref(), Some("aaaaaa ───"));
}

#[test]
fn every_ruler_starts_with_the_byte_looked_for_first() {
    for ruler in RULERS {
        assert_eq!(ruler.to_string().as_bytes()[0], FIRST_BYTE_OF_A_RULER);
    }
    assert_eq!(
        shorten("✓ → │ other characters that start with the same byte"),
        None
    );
}

fn longest_ruler(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    let mut previous = None;
    for character in text.chars() {
        run = if previous == Some(character) {
            run + 1
        } else {
            1
        };
        previous = Some(character);
        if RULERS.contains(&character) {
            longest = longest.max(run);
        }
    }
    longest
}

proptest! {
    #[test]
    fn only_shortens_rulers(text in "[─━═a\\- \n]{0,120}") {
        let plain = |text: &str| text.chars().filter(|drawn| !RULERS.contains(drawn)).collect::<String>();

        match shorten(&text) {
            Some(shorter) => {
                prop_assert!(longest_ruler(&text) > LONGEST_KEPT);
                prop_assert_eq!(longest_ruler(&shorter), LONGEST_KEPT);
                prop_assert_eq!(plain(&shorter), plain(&text));
                prop_assert_eq!(shorten(&shorter), None);
            }
            None => prop_assert!(longest_ruler(&text) <= LONGEST_KEPT),
        }
    }
}
