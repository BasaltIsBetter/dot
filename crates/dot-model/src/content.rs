use base64::display::Base64Display;
use base64::engine::general_purpose::STANDARD;
use dot_api::{Image, Prompt};
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub(crate) struct UserMessage<'a> {
    role: &'static str,
    content: UserContent<'a>,
}

struct UserContent<'a>(&'a Prompt);

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Block<'a> {
    Image { source: Source<'a> },
    Text { text: &'a str },
}

#[derive(Serialize)]
pub(crate) struct Source<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    media_type: &'static str,
    data: Encoded<'a>,
}

struct Encoded<'a>(&'a [u8]);

impl<'a> UserMessage<'a> {
    pub(crate) const fn of(prompt: &'a Prompt) -> Self {
        Self {
            role: "user",
            content: UserContent(prompt),
        }
    }
}

impl Serialize for UserContent<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Prompt { text, images } = self.0;
        if images.is_empty() {
            return serializer.serialize_str(text);
        }

        let mut blocks = serializer.serialize_seq(None)?;
        for image in images {
            blocks.serialize_element(&Block::image(image))?;
        }
        if !text.is_empty() {
            blocks.serialize_element(&Block::Text { text })?;
        }
        blocks.end()
    }
}

impl<'a> Block<'a> {
    pub(crate) fn image(image: &'a Image) -> Self {
        Self::Image {
            source: Source::of(image),
        }
    }
}

impl<'a> Source<'a> {
    fn of(image: &'a Image) -> Self {
        Self {
            kind: "base64",
            media_type: image.format().media_type(),
            data: Encoded(image.bytes()),
        }
    }
}

impl Serialize for Encoded<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&Base64Display::new(self.0, &STANDARD))
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-model/content.rs"]
mod tests;
