use std::thread;

use rfd::FileDialog;

use crate::shell::ChosenFolder;
use crate::worker::Mailbox;

/// Opens the desktop's folder chooser without holding the window: through the XDG portal, or
/// `zenity` where there is none.
pub(crate) fn choose_folder(prompt: &str, mailbox: impl Mailbox<ChosenFolder>) {
    let chooser = FileDialog::new().set_title(prompt);
    let opened = thread::Builder::new().spawn(move || {
        if let Some(folder) = chooser.pick_folder() {
            mailbox.deliver(ChosenFolder(folder));
        }
    });
    if let Err(error) = opened {
        tracing::warn!(%error, "the folder chooser could not open");
    }
}
