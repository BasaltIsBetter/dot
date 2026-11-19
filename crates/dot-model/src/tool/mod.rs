mod call;
mod definition;
mod result;
mod shown;

pub(crate) use self::call::call_of;
pub(crate) use self::definition::{CLAUDE_CODE_NOTE, TOOLS, system_prompt};
pub(crate) use self::result::ToolResults;
pub(crate) use self::shown::{described, relative, shown};

#[cfg(test)]
#[path = "../../../../tests/dot-model/tool/mod.rs"]
pub(crate) mod tests;
