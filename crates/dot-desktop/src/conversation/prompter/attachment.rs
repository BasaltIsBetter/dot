use dot_api::Image;
use dot_transcript::{TextWidth, columns, truncated};

const LABEL_GAP: &str = "  ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Attachment {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) size: String,
    pub(crate) image: Image,
}

impl Attachment {
    pub(crate) fn label(&self, width: TextWidth) -> String {
        let gaps = 2 * columns(LABEL_GAP);
        let kept = columns(&self.kind) + columns(&self.size) + gaps;
        let name = truncated(&self.name, width.columns().saturating_sub(kept));
        [self.kind.as_str(), &name, &self.size].join(LABEL_GAP)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/attachment.rs"]
pub(crate) mod tests;
