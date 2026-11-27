mod entry;
mod fade;
mod ghost;
mod history;
mod tool;
mod view;

pub use self::entry::Kind;
pub use self::fade::heat_after;
pub(crate) use self::history::Row;
pub use self::history::{Insertion, Transcript};
pub use self::tool::{Aside, Glyph, Running, Tool};
pub use self::view::{View, lines};
