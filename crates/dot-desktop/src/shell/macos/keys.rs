use winit::keyboard::ModifiersState;

use crate::event::Keystroke;

pub(crate) const COPY_KEY: &str = "⌘C";

pub(crate) const fn native_modifiers(modifiers: ModifiersState) -> ModifiersState {
    modifiers
}

pub(crate) const fn native_keystroke(keystroke: Keystroke) -> Keystroke {
    keystroke
}

/// The menu of the Mac quits Dot itself.
pub(crate) const fn quits(_keystroke: &Keystroke) -> bool {
    false
}
