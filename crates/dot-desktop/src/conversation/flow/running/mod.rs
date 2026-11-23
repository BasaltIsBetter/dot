mod lane;

use std::time::Instant;

use dot_api::{LIVE_OUTPUT_LINES, Text, Translator};

use crate::canvas::{Layout, Rectangle, TextSize};
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::revealed;
use crate::conversation::stage::Stage;
use dot_transcript::{Agent, Glyph, Ink, Line, Placed, Span, columns, index, truncated};

use self::lane::lane;

const OUTPUT_INDENT: &str = "  ";

#[derive(Clone, Copy, Debug)]
pub(super) enum RunningRow<'a> {
    Command(&'a str),
    Output(&'a str),
    Agents(u64),
    Lane(&'a str, &'a Agent),
}

#[derive(Debug, Default)]
pub(super) struct RunningScene {
    pub(super) regular: Vec<Placed>,
    pub(super) small: Vec<Placed>,
    pub(super) marks: Vec<Rectangle>,
}

impl Conversation {
    pub(super) fn running_rows(&self) -> Vec<RunningRow<'_>> {
        let mut rows = Vec::new();
        for running in self.transcript.running() {
            if running.tool.glyph != Glyph::Command {
                continue;
            }
            rows.push(RunningRow::Command(&running.tool.label));
            let shown = running.output.len().min(LIVE_OUTPUT_LINES);
            let last = &running.output[running.output.len() - shown..];
            rows.extend(last.iter().map(|line| RunningRow::Output(line)));
        }
        let working: Vec<(&str, &Agent)> = self.agents.working().collect();
        if !working.is_empty() {
            let count = u64::try_from(working.len()).unwrap_or(u64::MAX);
            rows.push(RunningRow::Agents(count));
            rows.extend(
                working
                    .into_iter()
                    .map(|(id, agent)| RunningRow::Lane(id, agent)),
            );
        }
        rows
    }

    pub(super) fn running_scene(
        &self,
        (stage, layout): (&Stage, &Layout),
        top_of: impl Fn(usize) -> f32,
        now: Instant,
        translator: &impl Translator,
    ) -> RunningScene {
        let mut scene = RunningScene::default();
        let regular_room = room(stage, layout, TextSize::Regular);
        for (row, running) in self.running_rows().into_iter().enumerate() {
            let row_top = top_of(row);
            let text_top = layout.text_top(TextSize::Regular, row_top, stage.row);
            match running {
                RunningRow::Command(label) => {
                    let text = format!("{} {label}", Glyph::Command.mark());
                    scene.regular.push(live(&text, regular_room, text_top));
                }
                RunningRow::Agents(count) => {
                    let text = format!(
                        "{} {}",
                        Glyph::Agents.mark(),
                        translator.count(Text::AgentsAtWork, count)
                    );
                    scene.regular.push(live(&text, regular_room, text_top));
                }
                RunningRow::Output(text) => {
                    let room =
                        room(stage, layout, TextSize::Small).saturating_sub(OUTPUT_INDENT.len());
                    let shown = format!("{OUTPUT_INDENT}{}", truncated(&revealed(text), room));
                    scene.small.push(Placed {
                        top: layout.text_top(TextSize::Small, row_top, stage.row),
                        line: Line::new(0, vec![Span::new(shown, Ink::Secondary)]),
                        clip: None,
                    });
                }
                RunningRow::Lane(_, agent) => {
                    let drawn = lane(agent, row_top, (stage, layout), now, translator);
                    scene.regular.push(drawn.text);
                    scene.small.push(drawn.rate);
                    scene.marks.extend(drawn.spark);
                }
            }
        }
        scene
    }

    pub(super) fn lane_at(&self, row: usize) -> Option<&str> {
        match self.running_rows().get(row)? {
            RunningRow::Lane(id, _) => Some(id),
            _ => None,
        }
    }
}

fn room(stage: &Stage, layout: &Layout, size: TextSize) -> usize {
    let [left, right] = stage.column;
    index(f64::from((right - left) / layout.advance_of(size)))
}

fn live(text: &str, room: usize, row_top: f32) -> Placed {
    Placed {
        top: row_top,
        line: Line::new(0, vec![Span::new(truncated(text, room), Ink::Live)]),
        clip: None,
    }
}

pub(super) fn columns_of(text: &str) -> u16 {
    u16::try_from(columns(text)).unwrap_or(u16::MAX)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/running/mod.rs"]
pub(crate) mod tests;
