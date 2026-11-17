use std::borrow::Cow;

use winit::error::EventLoopError;

use super::*;

const EXIT_CODE: i32 = 3;

struct UpperCaseTranslator;

impl Translator for UpperCaseTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Owned(text.id().to_uppercase())
    }
}

#[test]
fn names_each_error_with_its_own_text() {
    let errors = [
        (
            DesktopError::EventLoop(EventLoopError::ExitFailure(EXIT_CODE)),
            Text::ErrorEventLoop,
        ),
        (
            DesktopError::UnsupportedSurface,
            Text::ErrorUnsupportedSurface,
        ),
        (DesktopError::SurfaceLost, Text::ErrorSurfaceLost),
        (
            DesktopError::TextPreparation(glyphon::PrepareError::AtlasFull),
            Text::ErrorText,
        ),
        (
            DesktopError::TextDrawing(glyphon::RenderError::RemovedFromAtlas),
            Text::ErrorText,
        ),
    ];

    for (error, text) in errors {
        assert_eq!(error.text(), text);
    }
}

#[test]
fn displays_the_id_of_its_text() {
    assert_eq!(DesktopError::SurfaceLost.to_string(), "error-surface-lost");
}

#[test]
fn describes_an_error_without_cause_in_one_translated_line() {
    let description = DesktopError::SurfaceLost.describe(&UpperCaseTranslator);

    assert_eq!(description, "ERROR-SURFACE-LOST");
}

#[test]
fn describes_the_technical_cause_under_the_translated_line() {
    let cause = EventLoopError::ExitFailure(EXIT_CODE);
    let expected = format!("ERROR-EVENT-LOOP\n  {cause}");

    let description = DesktopError::EventLoop(cause).describe(&UpperCaseTranslator);

    assert_eq!(description, expected);
}
