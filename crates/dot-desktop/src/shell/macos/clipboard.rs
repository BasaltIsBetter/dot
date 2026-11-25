use dot_api::{Image, ImageError};
use objc2_app_kit::NSPasteboard;
use objc2_foundation::NSString;

const PLAIN_TEXT: &str = "public.utf8-plain-text";
const UNITS_PER_CHARACTER: usize = 2;
const IMAGE_TYPES: [&str; 4] = [
    "public.png",
    "public.jpeg",
    "com.compuserve.gif",
    "org.webmproject.webp",
];

pub(crate) fn pasted_text(most_characters: usize) -> Option<String> {
    let text = NSPasteboard::generalPasteboard().stringForType(&NSString::from_str(PLAIN_TEXT))?;
    let most_units = most_characters.saturating_mul(UNITS_PER_CHARACTER);
    if text.length() <= most_units {
        return Some(text.to_string());
    }
    let cut = text
        .rangeOfComposedCharacterSequenceAtIndex(most_units)
        .location;
    Some(text.substringToIndex(cut).to_string())
}

pub(crate) fn copy_text(text: &str) {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    if !pasteboard.setString_forType(&NSString::from_str(text), &NSString::from_str(PLAIN_TEXT)) {
        tracing::warn!("macOS refused the text copied from the editor");
    }
}

pub(crate) fn pasted_image() -> Option<Result<Vec<u8>, ImageError>> {
    let pasteboard = NSPasteboard::generalPasteboard();
    IMAGE_TYPES.iter().find_map(|kind| {
        let data = pasteboard.dataForType(&NSString::from_str(kind))?;
        if data.length() > Image::LARGEST {
            return Some(Err(ImageError::TooLarge));
        }
        Some(Ok(data.to_vec()))
    })
}
