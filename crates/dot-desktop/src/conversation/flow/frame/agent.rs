use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Layout, Pane, Scene, TextSize};
use crate::conversation::agent::{AgentView, agent_lines, fading_back};
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::name_of;
use crate::conversation::instrument::Cluster;
use crate::conversation::stage::Stage;
use crate::status_bar::Activity;
use dot_transcript::Placed;

impl Conversation {
    pub(crate) fn open_agent(&mut self, id: &str, now: Instant) {
        if self.agents.get(id).is_some() {
            self.agent_view = Some(AgentView::open(id.to_owned(), now));
            self.back_from_agent = None;
            self.selecting = None;
        }
    }

    pub(crate) fn leave(&mut self) {
        self.agent_view = None;
        self.back_from_agent = None;
        self.selecting = None;
    }

    pub(in crate::conversation::flow) fn close_agent(&mut self, now: Instant) {
        if self.agent_view.take().is_some() {
            self.back_from_agent = Some(now);
        }
    }

    pub(in crate::conversation::flow) fn scroll_agent(
        &mut self,
        rows_up: f64,
        now: Instant,
        translator: &impl Translator,
    ) {
        let Some(view) = self.agent_view.as_ref() else {
            return;
        };
        let rows = self.agents.get(&view.id).map_or(0, |agent| {
            agent_lines(agent, self.prompter.width(), now, translator).len()
        });
        if let Some(view) = self.agent_view.as_mut() {
            view.scroll_by(rows_up, rows.saturating_sub(1));
        }
    }

    pub(super) fn agent_scene(
        &self,
        view: &AgentView,
        (stage, layout): (&Stage, &Layout),
        now: Instant,
        translator: &impl Translator,
    ) -> Option<Scene> {
        let agent = self.agents.get(&view.id)?;
        let lines = agent_lines(agent, self.prompter.width(), now, translator);
        let last = lines.len().saturating_sub(1);
        let focus = f64::from(u32::try_from(last).unwrap_or(u32::MAX)) - view.scrolled();
        let top = stage.top_row(focus);
        let placed = lines
            .into_iter()
            .enumerate()
            .map(|(row, line)| Placed {
                top: stage.row_top(row, top),
                line,
                clip: None,
            })
            .collect();
        let band = stage.reading_band(0.0);
        let mut scene = Scene {
            panes: vec![Pane {
                left: stage.column[0],
                size: TextSize::Regular,
                top: 0.0,
                bottom: stage.floor,
                lines: placed,
            }],
            grounds: vec![band],
            veils: stage.veils(),
            animating: view.fading(now).is_some() || self.instrument.is_animating(now),
            ..Scene::default()
        };
        self.tint(&mut scene.panes, translator);
        if stage.gutter.is_some()
            && let Some(reading) = self.instrument.reading(now)
        {
            let cluster = Cluster::of(&reading, &band, stage, layout, translator);
            scene.panes.extend(cluster.panes);
            scene.marks.extend(cluster.marks);
        }
        if let Some(fading) = view.fading(now) {
            scene.fade_around(stage.middle(), fading.alpha, fading.scale);
        }
        Some(scene)
    }

    pub(super) fn fade_back(&self, scene: &mut Scene, stage: &Stage, now: Instant) {
        let Some(fading) = self
            .back_from_agent
            .and_then(|closed| fading_back(closed, now))
        else {
            return;
        };
        scene.fade_around(stage.middle(), fading.alpha, fading.scale);
        scene.animating = true;
    }

    pub(in crate::conversation::flow) fn agent_activity(
        &self,
        translator: &impl Translator,
    ) -> Option<Activity> {
        let view = self.agent_view.as_ref()?;
        let agent = self.agents.get(&view.id)?;
        let working = agent.ended.is_none();
        let state = if working {
            Text::StateAgentWorking
        } else {
            Text::StateAgentDone
        };
        Some(Activity {
            text: Some(translator.with(state, name_of(agent)).into_owned()),
            live: working,
            sessions: None,
            model_heat: 0,
            effort_heat: 0,
            gauge_tint: None,
            gauge_fill: None,
            name: None,
            name_tint: None,
        })
    }

    pub(in crate::conversation::flow) fn agent_is_fading(&self, now: Instant) -> bool {
        let opening = self
            .agent_view
            .as_ref()
            .is_some_and(|view| view.fading(now).is_some());
        let closing = self
            .back_from_agent
            .is_some_and(|closed| fading_back(closed, now).is_some());
        opening || closing
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/frame/agent.rs"]
mod tests;
