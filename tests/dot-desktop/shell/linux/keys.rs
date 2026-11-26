use super::*;

fn keystroke(key: NamedKey, modifiers: ModifiersState) -> Keystroke {
    Keystroke {
        key: Key::Named(key),
        text: None,
        modifiers: native_modifiers(modifiers),
    }
}

fn typed(text: &str, modifiers: ModifiersState) -> Keystroke {
    Keystroke {
        key: Key::Character(text.into()),
        text: Some(text.into()),
        modifiers: native_modifiers(modifiers),
    }
}

#[test]
fn ctrl_is_the_command_key() {
    let ctrl_shift = ModifiersState::CONTROL | ModifiersState::SHIFT;

    assert_eq!(
        native_modifiers(ctrl_shift),
        ModifiersState::SUPER | ModifiersState::SHIFT
    );
    assert_eq!(native_modifiers(ModifiersState::ALT), ModifiersState::ALT);
}

#[test]
fn ctrl_moves_and_erases_by_word() {
    for key in [
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Backspace,
        NamedKey::Delete,
    ] {
        let read = native_keystroke(keystroke(key, ModifiersState::CONTROL));

        assert_eq!(read.modifiers, ModifiersState::ALT, "{key:?}");
    }
}

#[test]
fn ctrl_and_an_arrow_up_or_down_stays_a_command() {
    for key in [NamedKey::ArrowUp, NamedKey::ArrowDown] {
        let read = native_keystroke(keystroke(key, ModifiersState::CONTROL));

        assert_eq!(read.modifiers, ModifiersState::SUPER, "{key:?}");
    }
}

#[test]
fn a_letter_writes_itself_only_without_ctrl_or_alt() {
    let written = native_keystroke(typed("f", ModifiersState::SHIFT));

    assert_eq!(written.text.as_deref(), Some("f"));
    for modifiers in [ModifiersState::CONTROL, ModifiersState::ALT] {
        assert_eq!(native_keystroke(typed("f", modifiers)).text, None);
    }
}

#[test]
fn a_control_character_is_no_text() {
    let mut delete = keystroke(NamedKey::Delete, ModifiersState::empty());
    delete.text = Some("\u{7f}".into());

    assert_eq!(native_keystroke(delete).text, None);
}

#[test]
fn ctrl_q_quits() {
    assert!(quits(&typed("q", ModifiersState::CONTROL)));
    assert!(!quits(&typed("q", ModifiersState::empty())));
    assert!(!quits(&typed("w", ModifiersState::CONTROL)));
}
