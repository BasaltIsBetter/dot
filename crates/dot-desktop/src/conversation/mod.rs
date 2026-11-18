mod flow;
mod prompter;
mod scroll;
mod transcript;
mod wrap;
mod zone;

use crate::canvas::Layout;

pub(crate) use self::flow::Conversation;
pub(crate) use self::prompter::intent;

const MARKER: &str = "› ";
const TEXT_INDENT: u16 = 2;
const TEXT_WIDTH: usize = (Layout::COLUMNS - TEXT_INDENT) as usize;
