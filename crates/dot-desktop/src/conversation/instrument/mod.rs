mod cluster;
mod crew;
mod mini;

use std::time::{Duration, Instant};

use dot_transcript::{Calibration, Measures, Meter, Motion, SETTLE, opacity};

pub(crate) use self::cluster::{Cluster, short};
pub(crate) use self::crew::{Crew, CrewLook};
pub(crate) use self::mini::{MINI_SPARK_HEIGHT, MINI_SPARK_WIDTH, mini_spark};

const APPEARING: Duration = Duration::from_millis(120);
const SWITCHING: Duration = Duration::from_millis(160);
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
const HOTTEST: u8 = u8::MAX;

#[derive(Debug, Default)]
pub(crate) struct Instrument {
    calibration: Calibration,
    meter: Option<Meter>,
    exchange: usize,
    past: Vec<(usize, Measures)>,
    appeared: Option<Instant>,
    cooled_from: Option<Instant>,
    shown: Shown,
    switched: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Shown {
    #[default]
    Current,
    Exchange(Option<usize>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reading {
    pub(crate) measures: Option<Measures>,
    pub(crate) heat: u8,
    pub(crate) alpha: u8,
    pub(crate) values_alpha: u8,
}

impl Instrument {
    pub(crate) fn start(&mut self, exchange: usize, now: Instant) {
        self.meter = Some(Meter::new(now));
        self.exchange = exchange;
        self.appeared.get_or_insert(now);
        self.cooled_from = None;
    }

    pub(crate) fn hear(&mut self, characters: usize, now: Instant) {
        if let Some(meter) = self.meter.as_mut() {
            meter.hear(characters, &self.calibration, now);
        }
    }

    pub(crate) fn hear_tokens(&mut self, tokens: u64, now: Instant) {
        if let Some(meter) = self.meter.as_mut() {
            meter.hear_tokens(tokens, now);
        }
    }

    pub(crate) fn end(&mut self, exact: Option<u64>, now: Instant) {
        let Some(meter) = self.meter.as_mut().filter(|meter| meter.is_live()) else {
            return;
        };
        meter.end(exact, now);
        let (characters, counted) = meter.counted();
        if let Some(tokens) = counted {
            self.calibration.learn(characters, tokens);
        }
        self.past.push((self.exchange, meter.measures(now)));
        self.cooled_from = Some(now);
    }

    pub(crate) fn follow(&mut self, shown: Shown, now: Instant) {
        if shown == self.shown {
            return;
        }
        self.shown = shown;
        self.switched = Some(now);
        if shown != Shown::Current {
            self.appeared.get_or_insert(now);
        }
    }

    pub(crate) fn current(&self, now: Instant) -> Option<Measures> {
        self.meter.as_ref().map(|meter| meter.measures(now))
    }

    pub(crate) fn reading(&self, now: Instant) -> Option<Reading> {
        let appeared = self.appeared?;
        let (measures, heat) = match self.shown {
            Shown::Current => (self.current(now), self.heat(now)),
            Shown::Exchange(exchange) => (exchange.and_then(|exchange| self.of(exchange, now)), 0),
        };
        Some(Reading {
            measures,
            heat,
            alpha: faded_in(appeared, APPEARING, now),
            values_alpha: self
                .switched
                .map_or(u8::MAX, |switched| faded_in(switched, SWITCHING, now)),
        })
    }

    pub(crate) fn next_change(&self, now: Instant) -> Option<Instant> {
        self.meter.as_ref()?.next_change(now)
    }

    pub(crate) fn is_animating(&self, now: Instant) -> bool {
        let within = |since: Option<Instant>, lasting: Duration| {
            since.is_some_and(|since| now.saturating_duration_since(since) < lasting)
        };
        let flashing = self
            .meter
            .as_ref()
            .is_some_and(|meter| meter.is_live() && meter.measures(now).spark.is_flashing());
        within(self.appeared, APPEARING)
            || within(self.switched, SWITCHING)
            || within(self.cooled_from, COOLING.duration())
            || flashing
    }

    fn of(&self, exchange: usize, now: Instant) -> Option<Measures> {
        if exchange == self.exchange && self.meter.as_ref().is_some_and(Meter::is_live) {
            return self.current(now);
        }
        self.past
            .iter()
            .rev()
            .find(|(measured, _)| *measured == exchange)
            .map(|(_, measures)| *measures)
    }

    fn heat(&self, now: Instant) -> u8 {
        let Some(cooled_from) = self.cooled_from else {
            return if self.meter.is_some() { HOTTEST } else { 0 };
        };
        let elapsed = now.saturating_duration_since(cooled_from);
        let cooled = COOLING.eased(elapsed).unwrap_or(1.0);
        opacity(1.0 - cooled)
    }
}

fn faded_in(since: Instant, lasting: Duration, now: Instant) -> u8 {
    let elapsed = now.saturating_duration_since(since);
    if elapsed >= lasting {
        return u8::MAX;
    }
    let progress = elapsed.as_secs_f64() / lasting.as_secs_f64();
    let settled = SETTLE
        .eased(SETTLE.duration().mul_f64(progress))
        .unwrap_or(1.0);
    opacity(settled)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/instrument/mod.rs"]
mod tests;
