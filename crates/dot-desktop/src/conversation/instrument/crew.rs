use std::time::{Duration, Instant};

use dot_api::{Text, Translator};

use crate::canvas::{Layout, Paint, Pane, Rectangle, Silhouette, TextSize};
use crate::conversation::instrument::Reading;
use crate::conversation::instrument::cluster::{Grid, VALUE_COLUMNS, share};
use crate::conversation::stage::Stage;
use crate::conversation::tint::GOOD;
use dot_transcript::{Agents, Ink, Line, Motion, OPAQUE, Placed, Span, columns, opacity};

const ROWS_ABOVE: f32 = 3.0;
const FIGURE_GAP: f32 = 3.0;
const LABEL_GAP: f32 = 8.0;
const MOST_FIGURES: usize = 5;
const MANY: char = '×';
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
const LABEL_INK: Ink = Ink::Secondary;
const VALUE_INK: Ink = Ink::Full;
const ENDED_INK: Ink = Ink::Faint;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Crew {
    figures: Vec<Figure>,
    working: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Figure {
    Working,
    Ended(Duration),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CrewLook<'a> {
    pub(crate) silhouette: &'a Silhouette,
    pub(crate) tool: Option<[u8; 3]>,
    pub(crate) states: bool,
    pub(crate) pulse: u8,
}

pub(crate) struct DrawnCrew {
    pub(crate) panes: Vec<Pane>,
    pub(crate) marks: Vec<Rectangle>,
}

impl Crew {
    pub(crate) fn of(agents: &Agents, now: Instant) -> Option<Self> {
        let cooled = agents
            .wave_ended()
            .is_some_and(|ended| !COOLING.lasts(now.saturating_duration_since(ended)));
        if cooled {
            return None;
        }
        let figures: Vec<Figure> = agents
            .wave()
            .map(|agent| {
                agent.ended.map_or(Figure::Working, |ended| {
                    Figure::Ended(now.saturating_duration_since(ended))
                })
            })
            .collect();
        let working = figures
            .iter()
            .filter(|&&figure| figure == Figure::Working)
            .count();
        (!figures.is_empty()).then_some(Self { figures, working })
    }

    pub(crate) fn is_cooling(agents: &Agents, now: Instant) -> bool {
        agents.wave().any(|agent| {
            agent
                .ended
                .is_some_and(|ended| COOLING.lasts(now.saturating_duration_since(ended)))
        })
    }

    pub(crate) fn drawn(
        &self,
        (reading, grid): (&Reading, &Grid),
        (stage, layout): (&Stage, &Layout),
        look: &CrewLook<'_>,
        translator: &impl Translator,
    ) -> DrawnCrew {
        let advance = layout.advance_of(TextSize::Small);
        let row_top = grid.first_top + grid.row * ROWS_ABOVE;
        let text_top = layout.text_top(TextSize::Small, row_top, grid.row);
        let gap = stage.points(FIGURE_GAP);
        let many = self.many();
        let many_width = many
            .as_deref()
            .map_or(0.0, |many| gap + width_of(many, advance));
        let shown = self.shown();
        let figures_width = to_f32(shown.len()) * (look.silhouette.size()[0] + gap) - gap;
        let figures_left = grid.label_right - many_width - figures_width;
        let marks = figures_drawn(
            shown,
            [figures_left, row_top],
            (grid.row, gap, stage.floor),
            (reading.alpha, look),
        );

        let faded = share(reading.alpha);
        let label = translator.translate(Text::InstrumentAgents);
        let label_left = figures_left - stage.points(LABEL_GAP) - width_of(&label, advance);
        let mut panes = vec![pane(
            label_left,
            line(&label, LABEL_INK, faded),
            (text_top, stage),
        )];
        if let Some(many) = many {
            let many_left = grid.label_right - width_of(&many, advance);
            panes.push(pane(
                many_left,
                line(&many, LABEL_INK, faded),
                (text_top, stage),
            ));
        }
        panes.push(pane(
            grid.value_left,
            self.count(reading),
            (text_top, stage),
        ));
        DrawnCrew { panes, marks }
    }

    fn many(&self) -> Option<String> {
        let total = self.figures.len();
        (total > MOST_FIGURES).then(|| format!("{MANY}{total}"))
    }

    fn count(&self, reading: &Reading) -> Line {
        let counted = format!("{}/{}", self.working, self.figures.len());
        let indent = u16::try_from(VALUE_COLUMNS.saturating_sub(columns(&counted))).unwrap_or(0);
        let value = Span::new(counted, VALUE_INK).heated(reading.heat);
        Line::new(indent, vec![value]).faded(share(reading.alpha) * share(reading.values_alpha))
    }

    fn shown(&self) -> &[Figure] {
        if self.figures.len() <= MOST_FIGURES {
            return &self.figures;
        }
        self.figures
            .iter()
            .min_by_key(|figure| match figure {
                Figure::Working => Duration::ZERO,
                Figure::Ended(ago) => *ago,
            })
            .map_or(&[], std::slice::from_ref)
    }
}

fn figures_drawn(
    shown: &[Figure],
    [left, row_top]: [f32; 2],
    (row, gap, floor): (f32, f32, f32),
    (alpha, look): (u8, &CrewLook<'_>),
) -> Vec<Rectangle> {
    let [width, height] = look.silhouette.size();
    let top = row_top + (row - height) / 2.0;
    let mut marks = Vec::new();
    for (at, &figure) in shown.iter().enumerate() {
        let paint = Paint::ink(ink_of(figure, look)).dimmed(alpha);
        let paint = match figure {
            Figure::Working => paint.dimmed(look.pulse),
            Figure::Ended(_) => paint,
        };
        let figure_left = left + to_f32(at) * (width + gap);
        marks.extend(look.silhouette.drawn([figure_left, top], paint));
    }
    marks.retain(|mark| mark.top + mark.height <= floor);
    marks
}

pub(crate) fn ink_of(figure: Figure, look: &CrewLook<'_>) -> Ink {
    let Figure::Ended(ago) = figure else {
        return look.tool.map_or(Ink::Live, Ink::Rgb);
    };
    match COOLING.eased(ago).filter(|_| look.states) {
        Some(cooled) => {
            let ended = ENDED_INK.rgb();
            Ink::Rgb(std::array::from_fn(|channel| {
                let from = f64::from(GOOD[channel]);
                let to = f64::from(ended[channel]);
                opacity((from + (to - from) * cooled) / f64::from(OPAQUE))
            }))
        }
        None => ENDED_INK,
    }
}

fn line(text: &str, ink: Ink, faded: f64) -> Line {
    Line::new(0, vec![Span::new(text, ink)]).faded(faded)
}

fn pane(left: f32, line: Line, (top, stage): (f32, &Stage)) -> Pane {
    Pane {
        left,
        size: TextSize::Small,
        top: 0.0,
        bottom: stage.floor,
        lines: vec![Placed {
            top,
            line,
            clip: None,
        }],
    }
}

fn width_of(text: &str, advance: f32) -> f32 {
    to_f32(columns(text)) * advance
}

fn to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/instrument/crew.rs"]
mod tests;
