#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod window;

use std::path::PathBuf;

#[cfg(target_os = "linux")]
use self::linux as native;
#[cfg(target_os = "macos")]
use self::macos as native;

pub(crate) use self::native::{
    COPY_KEY, choose_folder, copy_text, native_keystroke, native_modifiers, open_main_window,
    pasted_image, pasted_text, press_moves, quits, translate_menu,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChosenFolder(pub(crate) PathBuf);
