use std::time::Instant;

use crate::app::{DesktopApp, Parts};
use crate::keeping::KeptNews;
use crate::sessions::{Making, Wiring};

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn start_keeping(&mut self) {
        self.keeping.start(self.sessions.kept_ids());
    }

    pub(super) fn kept_news(&mut self, landed: KeptNews, now: Instant) {
        match landed {
            KeptNews::Restored(restored) => {
                let making = Making {
                    open: &mut self.open,
                    wiring: Wiring {
                        connector: &self.connector,
                        answering: (self.settings.permissions, self.settings.tuning),
                        mailbox: &self.mailbox,
                    },
                };
                self.sessions
                    .restore(restored, making, (now, self.translator));
                self.keeping.restored();
                self.refresh_status();
            }
            KeptNews::Prompts(older) => {
                self.recall.add_older(older);
                self.keeping.loaded();
            }
            KeptNews::Kept => {}
        }
    }

    pub(super) fn keep_what_changed(&mut self) {
        let arranged = self
            .keeping
            .last_arrangement()
            .is_some_and(|last| self.sessions.is_arranged_as(last));
        if self.keeping.has_restored() && !arranged {
            self.keeping.arrange(self.sessions.arrangement());
        }
        self.keeping
            .keep_prompts(self.recall.prompts(), self.recall.changes());
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/kept/mod.rs"]
mod tests;
