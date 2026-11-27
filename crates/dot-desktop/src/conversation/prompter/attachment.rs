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
        label(&self.kind, &self.name, &self.size, width)
    }
}

pub(crate) fn label(kind: &str, name: &str, size: &str, width: TextWidth) -> String {
    let gaps = 2 * columns(LABEL_GAP);
    let kept = columns(kind) + columns(size) + gaps;
    let name = truncated(name, width.columns().saturating_sub(kept));
    [kind, &name, size].join(LABEL_GAP)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/attachment.rs"]
pub(crate) mod tests;
