use std::borrow::Cow;

use winit::error::EventLoopError;

use super::*;

const EXIT_CODE: i32 = 3;

struct UpperCaseTranslator;

impl Translator for UpperCaseTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Owned(text.id().to_uppercase())
    }
    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
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

#[test]
fn a_problem_joins_the_chain_of_causes() {
    #[derive(Debug, thiserror::Error)]
    #[error("request failed")]
    struct RequestFailed(#[source] std::io::Error);
    let error = dot_api::ConnectError::caused_by(
        dot_api::ConnectFailure::AnthropicUnreachable,
        RequestFailed(std::io::Error::other("no route")),
    );

    let problem = Problem::new(Text::SignInAnthropicUnreachable, &error);

    assert_eq!(problem.text, Text::SignInAnthropicUnreachable);
    assert_eq!(problem.cause.as_deref(), Some("request failed: no route"));
}

#[test]
fn a_problem_without_cause_has_only_its_text() {
    let error = dot_api::ReplyError::new(dot_api::ReplyFailure::Overloaded);

    assert_eq!(
        Problem::new(Text::ReplyOverloaded, &error),
        Problem::plain(Text::ReplyOverloaded)
    );
}
