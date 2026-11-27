use crate::archive::ArchiveError;

/// Where Dot keeps the prompts the user sent, from every session, between two launches, so that
/// they can be written again.
pub trait PromptHistory: Send + Sync {
    /// Returns the prompts kept, oldest first.
    ///
    /// Reads the disk: call it off the interface thread. Prompts that cannot be read are left
    /// out.
    fn load(&self) -> Vec<String>;

    /// Keeps `prompts`, oldest first, in place of those kept before.
    ///
    /// Writes the disk: call it off the interface thread.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the prompts cannot be written.
    fn save(&self, prompts: &[String]) -> Result<(), ArchiveError>;
}
