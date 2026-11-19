mod flow;
mod prompter;
mod scroll;
mod transcript;
mod wrap;
mod zone;

pub(crate) use self::flow::{Command, Conversation};
pub(crate) use self::prompter::{Intent, intent};

const MARKER: &str = "› ";
const TEXT_INDENT: u16 = 2;
