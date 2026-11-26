use std::env;
use std::sync::Arc;

use dot_api::{Text, Translator};
use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;
use winit::platform::{wayland, x11};
use winit::window::{Window, WindowAttributes};

use crate::error::DesktopError;
use crate::shell::window::window_attributes;

const APPLICATION: &str = "dev.dot.desktop";
const INSTANCE: &str = "dot";
const WSL_VARIABLE: &str = "WSL_DISTRO_NAME";
const GRIP: f64 = 32.0;

pub(crate) fn open_main_window(
    event_loop: &ActiveEventLoop,
    translator: &impl Translator,
) -> Result<Arc<Window>, DesktopError> {
    let title = translator.translate(Text::WindowTitle);
    let window = Arc::new(event_loop.create_window(dressed(window_attributes(&title)))?);
    window.set_cursor_visible(false);
    Ok(window)
}

/// A Linux desktop gives Dot no menu bar: there is no menu to translate.
pub(crate) const fn translate_menu<T>(_translator: &T) {}

/// Moves a window that has no title bar when it is pressed along its top; returns whether the
/// press was for that.
pub(crate) fn press_moves(window: &Window, at: Option<PhysicalPosition<f64>>) -> bool {
    !window.is_decorated()
        && at.is_some_and(|at| is_on_the_grip(at, window.scale_factor()))
        && window.drag_window().is_ok()
}

fn is_on_the_grip(at: PhysicalPosition<f64>, scale_factor: f64) -> bool {
    at.y < GRIP * scale_factor
}

/// WSL's compositor crashes on the frame that winit draws around a window: there, Dot has none.
fn dressed(attributes: WindowAttributes) -> WindowAttributes {
    let attributes = attributes.with_decorations(env::var_os(WSL_VARIABLE).is_none());
    let attributes =
        wayland::WindowAttributesExtWayland::with_name(attributes, APPLICATION, INSTANCE);
    x11::WindowAttributesExtX11::with_name(attributes, APPLICATION, INSTANCE)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/shell/linux/window.rs"]
mod tests;
