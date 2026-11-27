use std::error::Error as _;
use std::fmt;

use dot_api::{Text, Translator};
use thiserror::Error;

const CAUSE_INDENT: &str = "  ";
const CAUSE_SEPARATOR: &str = ": ";

/// Error that keeps Dot's window from opening or drawing.
///
/// Its text lives in the translation files: [`DesktopError::text`] names it,
/// [`DesktopError::describe`] translates it, and `Display` writes its identifier.
#[derive(Debug, Error)]
pub enum DesktopError {
    /// The macOS event loop did not start, or stopped on an error.
    EventLoop(#[from] winit::error::EventLoopError),

    /// macOS refused to create the window.
    Window(#[from] winit::error::OsError),

    /// The window could not be attached to the GPU.
    Surface(#[from] wgpu::CreateSurfaceError),

    /// No Metal-compatible graphics card was found.
    Adapter(#[from] wgpu::RequestAdapterError),

    /// The graphics card refused to open a drawing session.
    Device(#[from] wgpu::RequestDeviceError),

    /// The graphics card found cannot draw into this window.
    UnsupportedSurface,

    /// The drawing surface was lost or no longer accepts frames.
    SurfaceLost,

    /// The text could not be prepared for the graphics card.
    TextPreparation(#[from] glyphon::PrepareError),

    /// The text could not be drawn.
    TextDrawing(#[from] glyphon::RenderError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Problem {
    pub(crate) text: Text,
    pub(crate) cause: Option<String>,
}

impl Problem {
    pub(crate) fn new(text: Text, error: &dyn std::error::Error) -> Self {
        let causes: Vec<String> = std::iter::successors(error.source(), |&cause| cause.source())
            .map(ToString::to_string)
            .collect();
        Self {
            text,
            cause: (!causes.is_empty()).then(|| causes.join(CAUSE_SEPARATOR)),
        }
    }

    pub(crate) const fn plain(text: Text) -> Self {
        Self { text, cause: None }
    }

    pub(crate) fn notice(&self, translator: &impl Translator) -> String {
        let text = translator.translate(self.text);
        match &self.cause {
            Some(cause) => format!("{text}\n{cause}"),
            None => text.into_owned(),
        }
    }
}

impl DesktopError {
    /// Returns the text that explains the error to the user.
    #[must_use]
    pub const fn text(&self) -> Text {
        match self {
            Self::EventLoop(_) => Text::ErrorEventLoop,
            Self::Window(_) => Text::ErrorWindow,
            Self::Surface(_) => Text::ErrorSurface,
            Self::Adapter(_) => Text::ErrorAdapter,
            Self::Device(_) => Text::ErrorDevice,
            Self::UnsupportedSurface => Text::ErrorUnsupportedSurface,
            Self::SurfaceLost => Text::ErrorSurfaceLost,
            Self::TextPreparation(_) | Self::TextDrawing(_) => Text::ErrorText,
        }
    }

    /// Returns the translated explanation of the error, followed by its technical causes, one
    /// per line and indented.
    ///
    /// The causes come from the libraries (winit, wgpu) and stay in their language, English.
    #[must_use]
    pub fn describe(&self, translator: &impl Translator) -> String {
        let explanation = translator.translate(self.text()).into_owned();
        let causes = std::iter::successors(self.source(), |&cause| cause.source())
            .map(|cause| format!("{CAUSE_INDENT}{cause}"));

        std::iter::once(explanation)
            .chain(causes)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl fmt::Display for DesktopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.text().id())
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/error.rs"]
mod tests;
