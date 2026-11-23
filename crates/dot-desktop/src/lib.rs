//! Dot's desktop interface (macOS): a native window, black, painted by the GPU.
//!
//! The entry point is [`run`], called once by the binary on the main thread. Every displayed
//! text (title, menu, sign-in screen, status bar, errors) goes through the given [`Translator`].
//! While nothing connects Dot to Claude, the window shows the sign-in screen: a Claude account
//! through Claude Code, an Anthropic account through `ant`, or an API key. Once connected, the
//! window holds the conversation with Claude, written in the prompter and answered through the
//! given [`Chat`], above a status bar that shows where the window's [`Session`] stands.

mod app;
#[cfg(feature = "bench")]
pub mod bench;
mod canvas;
mod conversation;
mod editor;
mod error;
mod event;
mod reply;
mod sessions;
mod shell;
mod sign_in;
mod status_bar;
mod worker;

use std::sync::Arc;

use dot_api::{Chat, Connector, PermissionMode, Session, Translator, Workspace};
use winit::event_loop::EventLoop;

use crate::app::{DesktopApp, OnMac};
use crate::event::Delivery;

pub use crate::error::DesktopError;

/// Opens Dot's window and blocks the thread until it closes.
///
/// The window has a native black background before it even appears, then the GPU paints it as
/// soon as macOS reports it visible. `connector` is asked in the background whether Dot is
/// connected to Claude; if not, the window offers to connect, and every slow step (keychain,
/// network, Claude Code) runs off the main thread.
///
/// Once connected, the window shows one session at a time, the others as cards in a column on
/// its right: `open` makes the session and the conversation of each one, the first when the
/// window opens, another each time the user adds one. The bar along its bottom shows the status
/// of the session shown, read in the background when the window opens and each time it comes
/// back to the front. Each prompt sent goes to the
/// session's conversation in the background, with the credential `connector` hands out, and the
/// reply streams in as it arrives, even while another session is shown; escape stops it.
///
/// Each action Claude wants to take on the Mac is put to the user in place of the prompter, or
/// allowed at once without asking, as `mode` says. A click on the name of a file Claude changed
/// opens it in a small editor, read and saved at each keystroke through `workspace`, off the
/// main thread. Background work is stopped when the window closes, with the red button or `⌘Q`.
///
/// # Errors
///
/// Returns [`DesktopError`] if the event loop, the window or the GPU fails to start, if the
/// drawing surface is lost along the way, or if the text cannot be drawn.
///
/// # Panics
///
/// Panics if called off the main thread: macOS only allows the event loop there.
pub fn run<T, K, S, H, W>(
    translator: &T,
    connector: K,
    open: impl FnMut() -> (S, H) + 'static,
    (mode, workspace): (PermissionMode, W),
) -> Result<(), DesktopError>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    W: Workspace + 'static,
{
    let event_loop = EventLoop::<Delivery>::with_user_event().build()?;
    let proxy = Arc::new(event_loop.create_proxy());
    let parts = (proxy, Arc::new(workspace));
    let mut app =
        DesktopApp::<OnMac<T, K, S, H, W>>::new(translator, connector, Box::new(open), mode, parts);
    event_loop.run_app(&mut app)?;
    app.into_result()
}

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
