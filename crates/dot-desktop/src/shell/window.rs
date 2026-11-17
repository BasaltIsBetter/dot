use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSColor, NSWindow};
use winit::dpi::LogicalSize;
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Theme, Window, WindowAttributes, WindowId};

const INITIAL_SIZE: LogicalSize<f64> = LogicalSize::new(1440.0, 900.0);
const MINIMUM_SIZE: LogicalSize<f64> = LogicalSize::new(900.0, 600.0);

pub(crate) fn main_window_attributes(title: &str) -> WindowAttributes {
    WindowAttributes::default()
        .with_title(title)
        .with_inner_size(INITIAL_SIZE)
        .with_min_inner_size(MINIMUM_SIZE)
        .with_theme(Some(Theme::Dark))
        .with_visible(false)
        .with_titlebar_transparent(true)
        .with_title_hidden(true)
        .with_fullsize_content_view(true)
}

pub(crate) fn paint_background_black(window: &Window, main_thread: MainThreadMarker) {
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

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/shell/window.rs"]
mod tests;
