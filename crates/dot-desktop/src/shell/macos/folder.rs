use std::path::PathBuf;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSModalResponse, NSModalResponseOK, NSOpenPanel};
use objc2_foundation::NSString;

use crate::shell::ChosenFolder;
use crate::worker::Mailbox;

pub(crate) fn choose_folder(prompt: &str, mailbox: impl Mailbox<ChosenFolder>) {
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let panel = NSOpenPanel::openPanel(main_thread);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    panel.setPrompt(Some(&NSString::from_str(prompt)));
    let chosen = panel.clone();
    let handler = RcBlock::new(move |response: NSModalResponse| {
        if response != NSModalResponseOK {
            return;
        }
        let path = chosen.URL().and_then(|url| url.path());
        if let Some(path) = path {
            mailbox.deliver(ChosenFolder(PathBuf::from(path.to_string())));
        }
    });
    panel.beginWithCompletionHandler(&handler);
}
