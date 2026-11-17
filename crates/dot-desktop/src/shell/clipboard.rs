use objc2_app_kit::NSPasteboard;
use objc2_foundation::NSString;

const PLAIN_TEXT: &str = "public.utf8-plain-text";

pub(crate) fn pasted_text() -> Option<String> {
    NSPasteboard::generalPasteboard()
        .stringForType(&NSString::from_str(PLAIN_TEXT))
        .map(|text| text.to_string())
}
