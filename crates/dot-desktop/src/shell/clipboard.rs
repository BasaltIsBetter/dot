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

pub(crate) fn pasted_image() -> Option<Vec<u8>> {
    let pasteboard = NSPasteboard::generalPasteboard();
    IMAGE_TYPES.iter().find_map(|kind| {
        pasteboard
            .dataForType(&NSString::from_str(kind))
            .map(|data| data.to_vec())
    })
}
