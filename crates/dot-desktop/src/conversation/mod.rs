mod agent;
mod blink;
mod flow;
mod instrument;
mod prompter;
mod stage;
mod suggestion;
mod tint;

pub(crate) use self::blink::{Blink, Caret};
pub(crate) use self::flow::{Command, Conversation, Recall, revealed};
pub(crate) use self::instrument::{MINI_SPARK_HEIGHT, MINI_SPARK_WIDTH, mini_spark};
pub(crate) use self::prompter::{
    Draft, Edit, Intent, LONGEST as LONGEST_PROMPT, Motion, intent, line_around, word_around,
};
pub(crate) use self::stage::Stage;
pub(crate) use self::tint::Colours;
#[cfg(feature = "bench")]
pub(crate) use self::tint::{Tints, tint};
