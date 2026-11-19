use base64::Engine;
use proptest::prelude::*;
use serde_json::{Value, json};

use super::*;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n pixels";

fn written(prompt: &Prompt) -> Value {
    serde_json::to_value(UserMessage::of(prompt)).unwrap()
}

fn png() -> Image {
    Image::from_bytes(PNG.to_vec()).unwrap()
}

#[test]
fn writes_a_prompt_without_image_as_plain_text() {
    let prompt = Prompt {
        text: "Salut".to_owned(),
        images: Vec::new(),
    };

    assert_eq!(
        written(&prompt),
        json!({"role": "user", "content": "Salut"})
    );
}

#[test]
fn puts_the_images_before_the_text() {
    let prompt = Prompt {
        text: "Et ça ?".to_owned(),
        images: vec![png(), png()],
    };

    let image = json!({"type": "image", "source": {"type": "base64", "media_type": "image/png",
        "data": STANDARD.encode(PNG)}});
    assert_eq!(
        written(&prompt),
        json!({"role": "user", "content": [image, image, {"type": "text", "text": "Et ça ?"}]})
    );
}

#[test]
fn sends_no_empty_text_with_images_alone() {
    let prompt = Prompt {
        text: String::new(),
        images: vec![png()],
    };

    let content = &written(&prompt)["content"];
    assert_eq!(content.as_array().map(Vec::len), Some(1));
    assert_eq!(content[0]["type"], "image");
}

proptest! {
    #[test]
    fn carries_any_image_bytes_exactly(tail in proptest::collection::vec(any::<u8>(), 0..300)) {
        let bytes = [PNG, tail.as_slice()].concat();
        let prompt = Prompt {
            text: "x".to_owned(),
            images: vec![Image::from_bytes(bytes.clone()).unwrap()],
        };

        let data = written(&prompt)["content"][0]["source"]["data"].as_str().unwrap().to_owned();

        prop_assert_eq!(STANDARD.decode(data).unwrap(), bytes);
    }
}
