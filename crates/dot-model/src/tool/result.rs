use dot_api::{ToolContent, ToolOutput};
use serde::Serialize;

use crate::content::Block;

#[derive(Serialize)]
pub(crate) struct ToolResults<'a> {
    role: &'static str,
    content: Vec<ResultBlock<'a>>,
}

#[derive(Serialize)]
struct ResultBlock<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    tool_use_id: &'a str,
    content: ResultContent<'a>,
    #[serde(skip_serializing_if = "is_false")]
    is_error: bool,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ResultContent<'a> {
    Text(&'a str),
    Image([Block<'a>; 1]),
}

impl<'a> ToolResults<'a> {
    pub(crate) fn of(results: &'a [(String, ToolOutput)]) -> Self {
        Self {
            role: "user",
            content: results
                .iter()
                .map(|(id, output)| ResultBlock {
                    kind: "tool_result",
                    tool_use_id: id,
                    content: match &output.content {
                        ToolContent::Text(text) => ResultContent::Text(text),
                        ToolContent::Image(image) => ResultContent::Image([Block::image(image)]),
                    },
                    is_error: output.is_error,
                })
                .collect(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if hands the field by reference"
)]
const fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/tool/result.rs"]
mod tests;
