use std::error::Error;
use std::fmt;

use crate::text::Text;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const JPEG_SIGNATURE: &[u8] = b"\xff\xd8\xff";
const GIF_SIGNATURES: [&[u8]; 2] = [b"GIF87a", b"GIF89a"];
const RIFF_SIGNATURE: &[u8] = b"RIFF";
const WEBP_SIGNATURE: &[u8] = b"WEBP";
const WEBP_SIGNATURE_AT: usize = 8;

/// What the user sends Claude in one go: text, images, or both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prompt {
    /// The text written in the prompter; empty when only images are sent.
    pub text: String,
    /// The images attached, in the order they were added.
    pub images: Vec<Image>,
}

/// Image Claude can read, checked by its content when it enters Dot.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    format: ImageFormat,
    bytes: Vec<u8>,
}

/// Encoding of an [`Image`], among those Claude reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// Portable Network Graphics, what macOS screenshots are.
    Png,
    /// JPEG photo.
    Jpeg,
    /// GIF image; Claude reads its first frame.
    Gif,
    /// WebP image.
    Webp,
}

/// Reason some bytes cannot be sent to Claude as an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// The bytes are not a PNG, JPEG, GIF or WebP image.
    Unsupported,
    /// The image is larger than [`Image::LARGEST`].
    TooLarge,
}

impl Image {
    /// Largest image Dot sends, in bytes: 5 MiB, the limit of the Claude API for one image.
    pub const LARGEST: usize = 5 * 1_024 * 1_024;

    /// Reads an image from its bytes, recognizing its format by its first bytes, never by a file
    /// name.
    ///
    /// # Errors
    ///
    /// Returns [`ImageError::TooLarge`] past [`Image::LARGEST`] bytes, and
    /// [`ImageError::Unsupported`] if the bytes do not start like a PNG, JPEG, GIF or WebP image.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::{Image, ImageError, ImageFormat};
    ///
    /// let png = b"\x89PNG\r\n\x1a\n rest of the image".to_vec();
    /// assert_eq!(Image::from_bytes(png).unwrap().format(), ImageFormat::Png);
    /// assert_eq!(Image::from_bytes(b"hello".to_vec()), Err(ImageError::Unsupported));
    /// ```
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ImageError> {
        if bytes.len() > Self::LARGEST {
            return Err(ImageError::TooLarge);
        }
        let format = ImageFormat::recognize(&bytes).ok_or(ImageError::Unsupported)?;
        Ok(Self { format, bytes })
    }

    /// Returns how the image is encoded.
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// Returns the encoded image, as it was read.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl ImageFormat {
    /// Returns the media type the Claude API expects for this format, such as `image/png`.
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }

    fn recognize(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(PNG_SIGNATURE) {
            return Some(Self::Png);
        }
        if bytes.starts_with(JPEG_SIGNATURE) {
            return Some(Self::Jpeg);
        }
        if GIF_SIGNATURES
            .iter()
            .any(|signature| bytes.starts_with(signature))
        {
            return Some(Self::Gif);
        }
        let webp = bytes.starts_with(RIFF_SIGNATURE)
            && bytes
                .get(WEBP_SIGNATURE_AT..)
                .is_some_and(|rest| rest.starts_with(WEBP_SIGNATURE));
        webp.then_some(Self::Webp)
    }
}

impl ImageError {
    /// Returns the text that explains to the user why the image is not sent.
    #[must_use]
    pub const fn text(self) -> Text {
        match self {
            Self::Unsupported => Text::ImageUnsupported,
            Self::TooLarge => Text::ImageTooLarge,
        }
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Image")
            .field("format", &self.format)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unsupported => "the file is not a PNG, JPEG, GIF or WebP image",
            Self::TooLarge => "the image is larger than 5 MiB",
        })
    }
}

impl Error for ImageError {}

#[cfg(test)]
#[path = "../../../../tests/dot-api/chat/prompt.rs"]
mod tests;
