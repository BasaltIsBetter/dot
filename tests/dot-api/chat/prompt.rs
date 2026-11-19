use proptest::prelude::*;

use super::*;

fn starting_with(signature: &[u8]) -> Vec<u8> {
    [signature, b" the rest of the image".as_slice()].concat()
}

#[test]
fn recognizes_each_format_by_its_first_bytes() {
    let formats = [
        (starting_with(PNG_SIGNATURE), ImageFormat::Png),
        (starting_with(JPEG_SIGNATURE), ImageFormat::Jpeg),
        (starting_with(b"GIF87a"), ImageFormat::Gif),
        (starting_with(b"GIF89a"), ImageFormat::Gif),
        (b"RIFF\x10\x00\x00\x00WEBPVP8 ".to_vec(), ImageFormat::Webp),
    ];

    for (bytes, format) in formats {
        assert_eq!(Image::from_bytes(bytes).unwrap().format(), format);
    }
}

#[test]
fn gives_each_format_its_media_type() {
    assert_eq!(ImageFormat::Png.media_type(), "image/png");
    assert_eq!(ImageFormat::Jpeg.media_type(), "image/jpeg");
    assert_eq!(ImageFormat::Gif.media_type(), "image/gif");
    assert_eq!(ImageFormat::Webp.media_type(), "image/webp");
}

#[test]
fn refuses_what_is_not_an_image_claude_reads() {
    let refused = [
        b"".to_vec(),
        b"hello".to_vec(),
        b"%PDF-1.7".to_vec(),
        b"II*\x00 a tiff image".to_vec(),
        b"RIFF\x10\x00\x00\x00WAVEfmt ".to_vec(),
        b"RIFF".to_vec(),
    ];

    for bytes in refused {
        assert_eq!(Image::from_bytes(bytes), Err(ImageError::Unsupported));
    }
}

#[test]
fn allows_five_mebibytes_like_the_claude_api() {
    assert_eq!(Image::LARGEST, 5_242_880);
}

#[test]
fn takes_an_image_up_to_five_mebibytes() {
    let mut largest = starting_with(PNG_SIGNATURE);
    largest.resize(Image::LARGEST, 0);
    let mut too_large = largest.clone();
    too_large.push(0);

    assert!(Image::from_bytes(largest).is_ok());
    assert_eq!(Image::from_bytes(too_large), Err(ImageError::TooLarge));
}

#[test]
fn keeps_the_bytes_untouched() {
    let bytes = starting_with(JPEG_SIGNATURE);

    let image = Image::from_bytes(bytes.clone()).unwrap();

    assert_eq!(image.bytes(), bytes.as_slice());
}

#[test]
fn shows_only_the_format_and_the_size_when_debugged() {
    let image = Image::from_bytes(starting_with(PNG_SIGNATURE)).unwrap();

    assert_eq!(format!("{image:?}"), "Image { format: Png, bytes: 30 }");
}

#[test]
fn explains_a_refused_image_in_english() {
    assert_eq!(
        ImageError::Unsupported.to_string(),
        "the file is not a PNG, JPEG, GIF or WebP image"
    );
    assert_eq!(
        ImageError::TooLarge.to_string(),
        "the image is larger than 5 MiB"
    );
    assert_eq!(
        ImageError::TooMany.to_string(),
        "the prompt carries 20 images or 20 MiB of images at most"
    );
}

#[test]
fn explains_each_refused_image_with_its_own_text() {
    assert_eq!(ImageError::Unsupported.text(), Text::ImageUnsupported);
    assert_eq!(ImageError::TooLarge.text(), Text::ImageTooLarge);
    assert_eq!(ImageError::TooMany.text(), Text::ImageTooMany);
}

fn png_of(length: usize) -> Image {
    let mut bytes = PNG_SIGNATURE.to_vec();
    bytes.resize(length.max(PNG_SIGNATURE.len()), 0);
    Image::from_bytes(bytes).unwrap()
}

#[test]
fn makes_room_for_images_up_to_twenty() {
    let attached = vec![png_of(16); Prompt::MOST_IMAGES - 1];

    assert_eq!(Prompt::room_for(&attached, &png_of(16)), Ok(()));
    let full = vec![png_of(16); Prompt::MOST_IMAGES];
    assert_eq!(
        Prompt::room_for(&full, &png_of(16)),
        Err(ImageError::TooMany)
    );
}

#[test]
fn makes_room_for_twenty_mebibytes_of_images_in_all() {
    let left = 100;
    let mut attached = vec![png_of(Image::LARGEST); 3];
    attached.push(png_of(Prompt::LARGEST_IMAGES - 3 * Image::LARGEST - left));

    assert_eq!(Prompt::room_for(&attached, &png_of(left)), Ok(()));
    assert_eq!(
        Prompt::room_for(&attached, &png_of(left + 1)),
        Err(ImageError::TooMany)
    );
}

#[test]
fn an_empty_prompt_has_room_for_one_image() {
    assert_eq!(Prompt::room_for(&[], &png_of(Image::LARGEST)), Ok(()));
}

#[test]
fn an_empty_prompt_has_neither_text_nor_image() {
    let prompt = Prompt::default();

    assert_eq!(prompt.text, "");
    assert_eq!(prompt.images, []);
}

proptest! {
    #[test]
    fn takes_only_recognized_images(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        if let Ok(image) = Image::from_bytes(bytes.clone()) {
            prop_assert_eq!(ImageFormat::recognize(&bytes), Some(image.format()));
        }
    }
}
