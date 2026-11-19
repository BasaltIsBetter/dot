use dot_api::{Image, ImageError};
use objc2_app_kit::NSPasteboard;
use objc2_foundation::NSString;

const PLAIN_TEXT: &str = "public.utf8-plain-text";
const IMAGE_TYPES: [&str; 4] = [
    "public.png",
    "public.jpeg",
    "com.compuserve.gif",
    "org.webmproject.webp",
];

pub(crate) fn pasted_text() -> Option<String> {
    NSPasteboard::generalPasteboard()
        .stringForType(&NSString::from_str(PLAIN_TEXT))
        .map(|text| text.to_string())
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
