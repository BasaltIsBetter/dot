//! Dot's desktop interface (macOS): a native window, black, painted by the GPU.
//!
//! The entry point is [`run`], called once by the binary on the main thread. Every displayed
//! text (title, menu, sign-in screen, errors) goes through the given [`Translator`]. While
//! nothing connects Dot to Claude, the window shows the sign-in screen: a Claude account through
//! Claude Code, or an API key.

mod app;
mod canvas;
mod error;
mod event;
mod shell;
mod sign_in;
mod worker;

use dot_api::{Connector, Translator};
use winit::event_loop::EventLoop;

use crate::app::DesktopApp;
use crate::sign_in::Report;

pub use crate::error::DesktopError;

/// Opens Dot's window and blocks the thread until it closes.
///
/// The window has a native black background before it even appears, then the GPU paints it as
/// soon as macOS reports it visible. `connector` is asked in the background whether Dot is
/// connected to Claude; if not, the window offers to connect, and every slow step (keychain,
/// network, Claude Code) runs off the main thread. Background work is stopped when the window
/// closes, with the red button or `⌘Q`.
///
/// # Errors
///
/// Returns [`DesktopError`] if the event loop, the window or the GPU fails to start, if the
/// drawing surface is lost along the way, or if the text cannot be drawn.
///
/// # Panics
///
/// Panics if called off the main thread: macOS only allows the event loop there.
pub fn run<K: Connector + 'static>(
    translator: &impl Translator,
    connector: K,
) -> Result<(), DesktopError> {
    let event_loop = EventLoop::<Report>::with_user_event().build()?;
    let mut app = DesktopApp::new(translator, connector, event_loop.create_proxy());
    event_loop.run_app(&mut app)?;
    app.into_result()
}
