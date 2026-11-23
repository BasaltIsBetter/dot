use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use crate::canvas::{Rectangle, Rgba};
use crate::sessions::card::{Card, State};
use crate::sessions::rail::QUIET;
use dot_transcript::Ink;

const BLINK: Duration = Duration::from_millis(1_400);
const LINE: f32 = 1.0;
const HERE: &str = "◉";
const WRITING: &str = "●";
const PAUSED: &str = "‖";
const UNREAD: &str = "◆";
const IDLE: &str = "○";

pub(super) fn glyph_of(card: &Card) -> (&'static str, Ink) {
    if card.here {
        return (HERE, Ink::Full);
    }
    match card.state {
        State::Writing => (WRITING, Ink::Live),
        State::Paused => (PAUSED, QUIET),
        State::Unread(_) => (UNREAD, Ink::Full),
        State::Idle => (IDLE, QUIET),
    }
}

pub(super) fn corner_colour(card: &Card, hovered: bool, now: Instant) -> Rgba {
    if card.here {
        return Rgba::ink(Ink::Full);
    }
    match card.state {
        State::Writing => Rgba::ink(Ink::Live),
        State::Unread(since) => blinking(now.saturating_duration_since(since)),
        _ if hovered => Rgba::ink(Ink::Full),
        _ => Rgba::ink(Ink::Receding),
    }
}

pub(super) fn corners(
    [left, top, width, height]: [f32; 4],
    arm: f32,
    colour: Rgba,
) -> [Rectangle; 8] {
    let [right, bottom] = [left + width, top + height];
    let rectangle = |left: f32, top: f32, width: f32, height: f32| Rectangle {
        left: left.round(),
        top: top.round(),
        width,
        height,
        color: colour,
    };
    [
        rectangle(left, top, arm, LINE),
        rectangle(left, top, LINE, arm),
        rectangle(right - arm, top, arm, LINE),
        rectangle(right - LINE, top, LINE, arm),
        rectangle(left, bottom - LINE, arm, LINE),
        rectangle(left, bottom - arm, LINE, arm),
        rectangle(right - arm, bottom - LINE, arm, LINE),
        rectangle(right - LINE, bottom - arm, LINE, arm),
    ]
}

fn blinking(elapsed: Duration) -> Rgba {
    let phase = (elapsed.as_secs_f64() % BLINK.as_secs_f64()) / BLINK.as_secs_f64();
    let lit = (1.0 - (TAU * phase).cos()) / 2.0;
    let [dark, light] = [Ink::Receding.rgb()[0], Ink::Full.rgb()[0]];
    let level = f64::from(dark) + (f64::from(light) - f64::from(dark)) * lit;
    Rgba {
        rgb: [to_level(level); 3],
        alpha: u8::MAX,
    }
}

pub(super) fn plus([x, y]: [f32; 2], arm: f32, colour: Rgba) -> [Rectangle; 2] {
    [
        Rectangle {
            left: (x - arm / 2.0).round(),
            top: y.round(),
            width: arm,
            height: LINE,
            color: colour,
        },
        Rectangle {
            left: x.round(),
            top: (y - arm / 2.0).round(),
            width: LINE,
            height: arm,
            color: colour,
        },
    ]
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a grey level or a count of cells lies within the 0 to 255 range"
)]
fn to_level(level: f64) -> u8 {
    level.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/mark.rs"]
mod tests;
