use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::ModifiersState;

use crate::sign_in::{KeyIntent, intent};

#[derive(Debug, PartialEq)]
pub(crate) enum WindowAction {
    Close,
    Resize(PhysicalSize<u32>),
    Rescale(f64),
    Paint,
    Modifiers(ModifiersState),
    Key(KeyIntent),
    Point(PhysicalPosition<f64>),
    Press,
    Ignore,
}

pub(crate) fn window_action(event: &WindowEvent, modifiers: ModifiersState) -> WindowAction {
    match event {
        WindowEvent::CloseRequested => WindowAction::Close,
        WindowEvent::Resized(size) => WindowAction::Resize(*size),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            WindowAction::Rescale(*scale_factor)
        }
        WindowEvent::RedrawRequested | WindowEvent::Occluded(false) => WindowAction::Paint,
        WindowEvent::ModifiersChanged(modifiers) => WindowAction::Modifiers(modifiers.state()),
        WindowEvent::KeyboardInput { event, .. } => key_action(event, modifiers),
        WindowEvent::CursorMoved { position, .. } => WindowAction::Point(*position),
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } => WindowAction::Press,
        _ => WindowAction::Ignore,
    }
}

fn key_action(event: &KeyEvent, modifiers: ModifiersState) -> WindowAction {
    if event.state != ElementState::Pressed {
        return WindowAction::Ignore;
    }
    intent(&event.logical_key, event.text.as_deref(), modifiers)
        .map_or(WindowAction::Ignore, WindowAction::Key)
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/event.rs"]
mod tests;
