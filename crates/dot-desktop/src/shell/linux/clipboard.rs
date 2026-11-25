use std::sync::{Mutex, PoisonError};

use arboard::{Clipboard, Error, ImageData};
use dot_api::{Image, ImageError};
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

const CHANNELS: usize = 4;

/// On Linux the program that copied serves what it copied: the clipboard stays open as long as
/// Dot runs.
static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

pub(crate) fn pasted_text(most_characters: usize) -> Option<String> {
    let text = on_clipboard(Clipboard::get_text)
        .inspect_err(|error| unless_empty(error, "the clipboard could not be read"))
        .ok()?;
    Some(at_most(text, most_characters))
}

pub(crate) fn copy_text(text: &str) {
    if let Err(error) = on_clipboard(|clipboard| clipboard.set_text(text)) {
        tracing::warn!(%error, "the desktop refused the text copied from Dot");
    }
}

pub(crate) fn pasted_image() -> Option<Result<Vec<u8>, ImageError>> {
    let pixels = on_clipboard(Clipboard::get_image)
        .inspect_err(|error| unless_empty(error, "the clipboard's image could not be read"))
        .ok()?;
    let encoded = encoded(&pixels)?;
    if encoded.len() > Image::LARGEST {
        return Some(Err(ImageError::TooLarge));
    }
    Some(Ok(encoded))
}

fn on_clipboard<R>(act: impl FnOnce(&mut Clipboard) -> Result<R, Error>) -> Result<R, Error> {
    let mut kept = CLIPBOARD.lock().unwrap_or_else(PoisonError::into_inner);
    match &mut *kept {
        Some(clipboard) => act(clipboard),
        None => act(kept.insert(Clipboard::new()?)),
    }
}

fn unless_empty(error: &Error, told: &str) {
    if !matches!(error, Error::ContentNotAvailable) {
        tracing::warn!(%error, "{told}");
    }
}

fn at_most(mut text: String, most_characters: usize) -> String {
    if let Some((cut, _)) = text.char_indices().nth(most_characters) {
        text.truncate(cut);
    }
    text
}

fn encoded(pixels: &ImageData<'_>) -> Option<Vec<u8>> {
    let width = u32::try_from(pixels.width).ok()?;
    let height = u32::try_from(pixels.height).ok()?;
    if pixels.bytes.len()
        != pixels
            .width
            .checked_mul(pixels.height)?
            .checked_mul(CHANNELS)?
    {
        return None;
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels.bytes, width, height, ExtendedColorType::Rgba8)
        .inspect_err(|error| tracing::warn!(%error, "the pasted image could not be encoded"))
        .ok()?;
    Some(png)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/shell/linux/clipboard.rs"]
mod tests;
