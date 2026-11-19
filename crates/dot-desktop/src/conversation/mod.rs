mod flow;
mod prompter;
mod scroll;
mod transcript;
mod wrap;
mod zone;

pub(crate) use self::flow::{Command, Conversation};
#[cfg(feature = "bench")]
pub(crate) use self::prompter::Edit;
pub(crate) use self::prompter::{Intent, intent};
#[cfg(feature = "bench")]
pub(crate) use self::wrap::wrap_characters;

const MARKER: &str = "› ";
const TEXT_INDENT: u16 = 2;
