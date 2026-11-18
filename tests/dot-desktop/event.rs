use winit::event::{DeviceId, Modifiers, TouchPhase};

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
        (WindowEvent::Focused(true), WindowAction::Focus),
        (WindowEvent::Focused(false), WindowAction::Blur),
        (
            WindowEvent::CursorLeft {
                device_id: DeviceId::dummy(),
            },
            WindowAction::Leave,
        ),
        (
            WindowEvent::ModifiersChanged(Modifiers::from(ModifiersState::SUPER)),
            WindowAction::Modifiers(ModifiersState::SUPER),
        ),
    ];

    for (event, action) in cases {
        assert_eq!(window_action(&event, none), action, "{event:?}");
    }
}

#[test]
fn maps_the_wheel_and_the_trackpad_to_a_vertical_scroll() {
    let none = ModifiersState::empty();
    let wheel = |delta| WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta,
        phase: TouchPhase::Moved,
    };

    assert_eq!(
        window_action(&wheel(MouseScrollDelta::LineDelta(4.0, -2.0)), none),
        WindowAction::Scroll(Wheel::Lines(-2.0))
    );
    assert_eq!(
        window_action(
            &wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                9.0, 12.5
            ))),
            none
        ),
        WindowAction::Scroll(Wheel::Pixels(12.5))
    );
}

#[test]
fn maps_composed_text_from_the_input_method() {
    let none = ModifiersState::empty();
    let cases = [
        (
            Ime::Commit("ê".to_owned()),
            WindowAction::Commit("ê".to_owned()),
        ),
        (
            Ime::Preedit("^".to_owned(), Some((0, 1))),
            WindowAction::Compose("^".to_owned()),
        ),
        (Ime::Disabled, WindowAction::Compose(String::new())),
        (Ime::Enabled, WindowAction::Ignore),
    ];

    for (ime, action) in cases {
        assert_eq!(window_action(&WindowEvent::Ime(ime), none), action);
    }
}

#[test]
fn wraps_what_background_work_delivers() {
    let status = SessionStatus {
        model: dot_api::Model::OPUS_5_5,
        project: None,
        branch: None,
        usage: dot_api::Usage::default(),
        limits: dot_api::Limits::default(),
    };

    assert!(matches!(
        Delivery::from(Report::KeyChecked(Ok(()))),
        Delivery::SignIn(Report::KeyChecked(Ok(())))
    ));
    assert!(matches!(
        Delivery::from(status.clone()),
        Delivery::Session(delivered) if delivered == status
    ));
}
