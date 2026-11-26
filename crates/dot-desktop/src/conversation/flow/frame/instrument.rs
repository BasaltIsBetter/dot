use std::borrow::Cow;
use std::time::Instant;

use dot_api::Translator;

use crate::canvas::{Layout, Pane, Rectangle, Silhouette};
use crate::conversation::flow::Conversation;
use crate::conversation::instrument::{Cluster, Crew, CrewLook};
use crate::conversation::stage::Stage;

impl Conversation {
    pub(super) fn keep_the_silhouette(&mut self, layout: &Layout) {
        if !self.silhouette.is_at(layout.scale()) {
            self.silhouette = Silhouette::at(layout.scale());
        }
    }

    pub(super) fn silhouette_for(&self, layout: &Layout) -> Cow<'_, Silhouette> {
        if self.silhouette.is_at(layout.scale()) {
            return Cow::Borrowed(&self.silhouette);
        }
        Cow::Owned(Silhouette::at(layout.scale()))
    }

    pub(super) fn instrument_drawn(
        &self,
        band: &Rectangle,
        (stage, layout): (&Stage, &Layout),
        silhouette: &Silhouette,
        now: Instant,
        translator: &impl Translator,
    ) -> Option<(Vec<Pane>, Vec<Rectangle>)> {
        stage.gutter?;
        let reading = self.instrument.reading(now)?;
        let cluster = Cluster::of(&reading, band, stage, layout, translator);
        let mut panes = Vec::from(cluster.panes);
        let mut marks = cluster.marks;
        if let Some(crew) = Crew::of(&self.agents, now) {
            let look = CrewLook {
                silhouette,
                tool: self.colours.tool,
                states: self.colours.states,
                pulse: self.pulse.alpha(now),
            };
            let drawn = crew.drawn(
                (&reading, &cluster.grid),
                (stage, layout),
                &look,
                translator,
            );
            panes.extend(drawn.panes);
            marks.extend(drawn.marks);
        }
        Some((panes, marks))
    }

    pub(super) fn crew_is_cooling(&self, now: Instant) -> bool {
        Crew::is_cooling(&self.agents, now)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/frame/instrument.rs"]
mod tests;
