mod agent;
mod help;
mod instrument;

use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Bounds, Layout, Pane, Scene, TextSize};
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::Words;
use crate::conversation::instrument::Shown;
use crate::conversation::stage::Stage;
use crate::conversation::tint::{Colours, Tints, tint};
use dot_transcript::{Placed, TEXT_INDENT, View, index, lines, rows, to_f32};

pub(super) use self::help::Help;

const HELP_LEFT: f32 = 24.0;
const HELP_BELOW_CENTRE: f32 = 56.0;
const HELP_ABOVE_THE_BAR: f32 = 64.0;
const HELP_ROW: f32 = 22.0;

impl Conversation {
    pub(crate) const fn hold_caret(&mut self, steady: bool) {
        self.blink.hold(steady);
    }

    pub(crate) const fn colour(&mut self, colours: Colours) {
        self.colours = colours;
    }

    pub(crate) fn prepare(&mut self, layout: &Layout, now: Instant, focused: bool) {
        self.fit(layout, now);
        self.transcript.grow_code(now);
        self.pace_suggestions(focused, now);
        self.blink.pace(focused, now);
        for insertion in self.transcript.take_insertions() {
            self.reader.shift(insertion.at, insertion.rows);
        }
        let stage = Stage::of(layout);
        self.reader.settle(self.plan(&stage, now), now);
        self.follow_edges(layout, now);
        self.watch_lanes(&stage, now);
        let working = self.agents.working().next().is_some();
        self.pulse.pace(focused && working, now);
        self.keep_the_silhouette(layout);
        let at_rest = !self.transcript.is_live() && self.reader.is_centred(now);
        self.help.pace(at_rest, focused, now);
        let shown = if self.reader.is_reviewing() {
            let row = index(self.reader.focus(now).round());
            Shown::Exchange(self.transcript.exchange_at(row))
        } else {
            Shown::Current
        };
        self.instrument.follow(shown, now);
    }

    pub(crate) fn scene(
        &self,
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) -> Scene {
        let stage = Stage::of(layout);
        if let Some(scene) = self
            .agent_view
            .as_ref()
            .and_then(|view| self.agent_scene(view, (&stage, layout), now, translator))
        {
            return scene;
        }
        let view = self.view(&stage, now);
        let mut flow = lines(&self.transcript, &view, now, &Words(translator));
        flow.extend(self.below_lines(&stage, view.top, now, translator));
        let silhouette = self.silhouette_for(layout);
        let silhouette = silhouette.as_ref();
        let running = self.below(now).running.map(|first| {
            let top_of = |row: usize| stage.row_top(first + row, view.top);
            self.running_scene((&stage, layout), top_of, silhouette, now, translator)
        });

        let band = stage.reading_band(self.reader.band(now));
        let mut marks = Vec::new();
        if !self.replied && !self.reader.is_reviewing() {
            marks.push(stage.tick(&band));
        }
        if let Some(running) = &running {
            flow.extend(running.regular.iter().cloned());
        }
        let mut panes = vec![Pane {
            left: stage.column[0],
            size: TextSize::Regular,
            top: 0.0,
            bottom: stage.floor,
            lines: flow,
        }];
        if let Some(running) = running {
            marks.extend(running.marks);
            panes.push(Pane {
                left: stage.column[0],
                size: TextSize::Small,
                top: 0.0,
                bottom: stage.floor,
                lines: running.small,
            });
        }
        self.tint(&mut panes, translator);
        let drawn = (&mut panes, &mut marks);
        self.draw_marker(drawn, (&stage, layout), silhouette, now, translator);
        let beside = self.instrument_drawn(&band, (&stage, layout), silhouette, now, translator);
        if let Some((beside_panes, beside_marks)) = beside {
            panes.extend(beside_panes);
            marks.extend(beside_marks);
        }
        panes.extend(self.help_pane(&stage, layout, now, translator));
        let selected = self.selection_drawn((&stage, layout), view.top, now, translator);
        panes.extend(selected.panes);
        marks.extend(selected.marks);
        let mut grounds = vec![band];
        grounds.extend(selected.grounds);

        let mut scene = Scene {
            panes,
            grounds,
            marks,
            veils: stage.veils(),
            animating: self.is_animating(now),
            ..Scene::default()
        };
        self.fade_back(&mut scene, &stage, now);
        scene
    }

    pub(super) fn tint(&self, panes: &mut [Pane], translator: &impl Translator) {
        let stopped = translator.translate(Text::ToolStopped);
        let tints = Tints {
            tool: self.colours.tool,
            states: self.colours.states,
            stopped: &stopped,
        };
        tint(panes, &tints);
    }

    pub(crate) fn next_change(&self, now: Instant) -> Option<Instant> {
        let suggestion = self.suggestions.next_change(now);
        let blink = self.blink.next_change(now);
        let help = self.help.wakes_at(now);
        let instrument = self.instrument.next_change(now);
        let curtain = self.curtain_opens_a_row_at(now);
        let pulse = self.pulse.next_change(now);
        suggestion
            .into_iter()
            .chain(blink)
            .chain(help)
            .chain(instrument)
            .chain(curtain)
            .chain(pulse)
            .min()
    }

    pub(crate) fn caret_area(&self, layout: &Layout, now: Instant) -> Bounds {
        let stage = Stage::of(layout);
        let top = stage.top_row(self.reader.focus(now));
        let (_, column) = self.prompter.caret_cell();
        let column = u16::try_from(column).unwrap_or(u16::MAX);
        Bounds {
            left: layout.x(TEXT_INDENT.saturating_add(column)),
            top: stage.row_top(self.caret_row(now), top),
            width: layout.advance(),
            height: stage.row,
        }
    }

    fn view(&self, stage: &Stage, now: Instant) -> View {
        let focus = self.reader.focus(now);
        View {
            band: stage.band(),
            line_height: stage.row,
            top: stage.top_row(focus),
            review: self.reader.is_reviewing().then_some(focus),
            caret_lit: self.blink.caret(now).is_shown()
                && self.prompter.is_empty()
                && self.pause.is_none(),
            hovered: self.pointer.and_then(|[_, y]| stage.row_at(y, focus)),
        }
    }

    fn is_animating(&self, now: Instant) -> bool {
        let suggestion_rolls = !self.transcript.is_live()
            && self.prompter.is_empty()
            && self.suggestions.is_rolling(now);
        self.transcript.is_animating(now)
            || self.reader.is_moving(now)
            || self.prompter.is_cooling(now)
            || self.help.is_fading(now)
            || self.instrument.is_animating(now)
            || self.crew_is_cooling(now)
            || self.marker.is_fading(now)
            || self.agent_is_fading(now)
            || self.selection_moves(now)
            || self.is_pacing()
            || suggestion_rolls
    }

    fn pace_suggestions(&mut self, focused: bool, now: Instant) {
        let idle = self.question.is_none() && !self.transcript.is_live();
        if focused && idle && self.prompter.is_empty() {
            self.suggestions.turn(now);
        } else {
            self.suggestions.hold(now);
        }
    }

    fn help_pane(
        &self,
        stage: &Stage,
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) -> Option<Pane> {
        let lines = self.help.lines(translator, now);
        if lines.is_empty() {
            return None;
        }
        let row = stage.points(HELP_ROW);
        let first = if stage.gutter.is_some() {
            stage.centre + stage.points(HELP_BELOW_CENTRE)
        } else {
            let height = to_f32(rows(lines.len())) * row;
            stage.floor - stage.points(HELP_ABOVE_THE_BAR) - height
        };
        let placed = lines
            .into_iter()
            .zip(0_u16..)
            .map(|(line, index)| Placed {
                top: layout.text_top(TextSize::Small, first + f32::from(index) * row, row),
                line,
                clip: None,
            })
            .collect();
        Some(Pane {
            left: stage.points(HELP_LEFT),
            size: TextSize::Small,
            top: 0.0,
            bottom: stage.floor,
            lines: placed,
        })
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/frame/mod.rs"]
mod tests;
