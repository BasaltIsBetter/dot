use winit::dpi::LogicalSize;
use winit::window::{Theme, Window, WindowAttributes};

pub(super) const INITIAL_SIZE: LogicalSize<f64> = LogicalSize::new(1000.0, 700.0);
pub(super) const MINIMUM_SIZE: LogicalSize<f64> = LogicalSize::new(480.0, 320.0);

pub(super) fn window_attributes(title: &str) -> WindowAttributes {
    Window::default_attributes()
        .with_title(title)
        .with_inner_size(INITIAL_SIZE)
        .with_min_inner_size(MINIMUM_SIZE)
        .with_theme(Some(Theme::Dark))
        .with_visible(false)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/shell/window.rs"]
mod tests;
