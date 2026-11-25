use std::sync::Arc;

use dot_api::{Text, Translator};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSColor, NSWindow};
use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::error::DesktopError;
use crate::shell::macos::menu::translate_menu;
use crate::shell::window::window_attributes;

const OPENED_ON_THE_MAIN_THREAD: &str = "winit opens the window on the main thread";

pub(crate) fn open_main_window(
    event_loop: &ActiveEventLoop,
    translator: &impl Translator,
) -> Result<Arc<Window>, DesktopError> {
    let main_thread = MainThreadMarker::new().expect(OPENED_ON_THE_MAIN_THREAD);
    let title = translator.translate(Text::WindowTitle);
    let window = Arc::new(event_loop.create_window(main_window_attributes(&title))?);
    window.set_cursor_visible(false);

    translate_menu(translator);
    paint_background_black(&window, main_thread);
    Ok(window)
}

/// macOS moves the window by its title bar itself.
pub(crate) const fn press_moves(_window: &Window, _at: Option<PhysicalPosition<f64>>) -> bool {
    false
}

fn main_window_attributes(title: &str) -> WindowAttributes {
    window_attributes(title)
        .with_titlebar_transparent(true)
        .with_title_hidden(true)
        .with_fullsize_content_view(true)
}

fn paint_background_black(window: &Window, main_thread: MainThreadMarker) {
    let black = NSColor::blackColor();
    let native_windows = NSApplication::sharedApplication(main_thread).windows();

    native_windows
        .iter()
        .filter(|native_window| is_native_window_of(native_window, window.id()))
        .for_each(|native_window| native_window.setBackgroundColor(Some(&black)));
}

fn is_native_window_of(native_window: &NSWindow, id: WindowId) -> bool {
    let address = std::ptr::from_ref(native_window).addr();
    u64::try_from(address).is_ok_and(|address| WindowId::from(address) == id)
}
