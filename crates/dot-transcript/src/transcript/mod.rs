mod entry;
mod fade;
mod ghost;
mod history;
mod tool;
mod view;

pub use self::entry::Kind;
pub use self::fade::heat_after;
pub use self::history::{Insertion, Transcript};
pub use self::tool::{Glyph, Running, Tool};
pub use self::view::{Marked, View, gutter, lines};
