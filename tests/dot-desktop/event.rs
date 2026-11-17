use winit::event::Modifiers;

use super::*;

const RETINA_SIZE: PhysicalSize<u32> = PhysicalSize::new(2880, 1800);

#[test]
fn maps_window_events_to_actions() {
    let none = ModifiersState::empty();
    let cases = [
        (WindowEvent::CloseRequested, WindowAction::Close),
        (
            WindowEvent::Resized(RETINA_SIZE),
            WindowAction::Resize(RETINA_SIZE),
        ),
        (WindowEvent::RedrawRequested, WindowAction::Paint),
        (WindowEvent::Occluded(false), WindowAction::Paint),
        (WindowEvent::Occluded(true), WindowAction::Ignore),
        (WindowEvent::Focused(true), WindowAction::Ignore),
        (
            WindowEvent::ModifiersChanged(Modifiers::from(ModifiersState::SUPER)),
            WindowAction::Modifiers(ModifiersState::SUPER),
        ),
    ];

    for (event, action) in cases {
        assert_eq!(window_action(&event, none), action, "{event:?}");
    }
}
