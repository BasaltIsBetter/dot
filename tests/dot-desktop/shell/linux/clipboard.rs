use std::borrow::Cow;

use super::*;

fn pixels(width: usize, height: usize, bytes: usize) -> ImageData<'static> {
    ImageData {
        width,
        height,
        bytes: Cow::Owned(vec![0x7f; bytes]),
    }
}

#[test]
fn a_pasted_image_becomes_a_png() {
    let png = encoded(&pixels(2, 3, 24)).unwrap();

    assert!(Image::from_bytes(png).is_ok());
}

#[test]
fn an_image_that_lies_about_its_size_is_left_out() {
    assert_eq!(encoded(&pixels(2, 3, 4)), None);
}

#[test]
fn a_text_is_cut_between_two_characters() {
    assert_eq!(at_most("éàü".to_owned(), 2), "éà");
    assert_eq!(at_most("éàü".to_owned(), 3), "éàü");
    assert_eq!(at_most(String::new(), 3), "");
}
