mod agent;
mod help;

use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Bounds, Icon, Layout, Paint, Pane, Rectangle, Scene, TextSize};
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::Words;
use crate::conversation::instrument::{Cluster, Shown};
use crate::conversation::stage::Stage;
use crate::conversation::tint::{Colours, Tints, tint};
use dot_transcript::{Ink, Marked, Placed, TEXT_INDENT, View, gutter, index, lines, rows, to_f32};

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
        let running = self.below(now).running.map(|first| {
            let top_of = |row: usize| stage.row_top(first + row, view.top);
            self.running_scene((&stage, layout), top_of, now, translator)
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
        if let Some([left, right]) = stage.gutter {
            let columns = index(f64::from(
                (right - left) / layout.advance_of(TextSize::Small),
            ));
            let marked = gutter(&self.transcript, &view, columns, now);
            let tool = self.colours.tool;
            marks.extend(
                marked
                    .iter()
                    .flat_map(|mark| glyph_of(mark, (left, tool), &stage, layout)),
            );
            panes.push(gutter_pane(&marked, left, &stage, layout));
        }
        self.tint(&mut panes, translator);
        if stage.gutter.is_some()
            && let Some(reading) = self.instrument.reading(now)
        {
            let cluster = Cluster::of(&reading, &band, &stage, layout, translator);
            panes.extend(cluster.panes);
            marks.extend(cluster.marks);
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
        suggestion
            .into_iter()
            .chain(blink)
            .chain(help)
            .chain(instrument)
            .chain(curtain)
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

fn gutter_pane(marked: &[Marked], left: f32, stage: &Stage, layout: &Layout) -> Pane {
    let lines = marked
        .iter()
        .map(|mark| Placed {
            top: layout.text_top(TextSize::Small, mark.top, stage.row),
            line: mark.line.clone(),
            clip: None,
        })
        .collect();
    Pane {
        left,
        size: TextSize::Small,
        top: 0.0,
        bottom: stage.floor,
        lines,
    }
}

fn glyph_of(
    mark: &Marked,
    (left, tool): (f32, Option<[u8; 3]>),
    stage: &Stage,
    layout: &Layout,
) -> impl Iterator<Item = Rectangle> {
    let dot = layout.icon_dot();
    let top = (mark.top + (stage.row - Icon::height() * dot) / 2.0).round();
    let ink = tool.map_or(Ink::Full, Ink::Rgb);
    let color = Paint::shaded(ink, mark.heat, mark.line.alpha);
    let floor = stage.floor;
    Icon::Tool(mark.glyph)
        .drawn([left, top], dot, color)
        .filter(move |dot| dot.top + dot.height <= floor)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/frame/mod.rs"]
mod tests;
