mod clipboard;
mod menu;
mod window;

pub(crate) use self::clipboard::pasted_text;
pub(crate) use self::menu::translate_menu;
pub(crate) use self::window::{main_window_attributes, paint_background_black};
