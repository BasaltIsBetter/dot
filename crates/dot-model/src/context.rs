use serde::{Deserialize, Serialize};

use crate::pricing::Price;

const KEPT_BY_A_SUMMARY: u64 = 26_000;
const READ_AGAIN_AFTER_A_SUMMARY: u64 = 20_000;
const MOST_KEPT_BY_A_SUMMARY: u64 = 100_000;
const REWRITTEN_AFTER_A_SUMMARY_PER_MILLE: u64 = 780;
const PER_MILLE: u64 = 1_000;
const SUMMARY_TOKENS: u64 = 6_000;
const SMALLEST_WINDOW: u64 = 100_000;
const SUMMARIZED_THIS_FAR_BELOW_THE_WINDOW: u64 = 33_000;
const ROOM_OVER_A_SUMMARY_NUMERATOR: u64 = 5;
const ROOM_OVER_A_SUMMARY_DENOMINATOR: u64 = 4;
const REQUESTS_BEFORE_A_WINDOW: u64 = 10;
const SLOWEST_GROWTH: u64 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Ledger {
    base: u64,
    context: u64,
    requests: u64,
    growth: u64,
    surplus: u128,
    cache_lasts_five_minutes: bool,
    summarized: Option<Summarized>,
    awaits_the_request_after_a_summary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) read: u64,
    pub(crate) written: u64,
    pub(crate) written_for_an_hour: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Summarized {
    context: u64,
    written: u64,
}

impl Ledger {
    pub(crate) const NOTHING_YET: Self = Self {
        base: 0,
        context: 0,
        requests: 0,
        growth: 0,
        surplus: 0,
        cache_lasts_five_minutes: false,
        summarized: None,
        awaits_the_request_after_a_summary: false,
    };

    pub(crate) fn request(&mut self, request: Request) {
        if request.read == 0 {
            return;
        }
        let opens_the_conversation = self.requests == 0;
        if opens_the_conversation {
            self.base = request.read;
        }
        if self.awaits_the_request_after_a_summary {
            self.awaits_the_request_after_a_summary = false;
            self.surplus = 0;
            self.summarized = Some(Summarized {
                context: request.read,
                written: request.written,
            });
        } else if !opens_the_conversation {
            let grown = request.read.saturating_sub(self.context);
            self.growth = self.growth.saturating_add(grown);
        }
        if request.written > 0 {
            self.cache_lasts_five_minutes = request.written_for_an_hour == 0;
        }

        let over = request.read.saturating_sub(self.floor());
        self.surplus = self.surplus.saturating_add(u128::from(over));
        self.context = request.read;
        self.requests = self.requests.saturating_add(1);
    }

    pub(crate) const fn summarized(&mut self) {
        self.awaits_the_request_after_a_summary = true;
    }

    pub(crate) fn summary_is_due(&self, price: &Price) -> bool {
        self.has_seen_enough()
            && self.surplus >= self.worth_of_a_summary(self.context, price)
            && self.context >= self.lowest_window()
    }

    pub(crate) fn window(&self, price: &Price, largest: u64) -> Option<u64> {
        if !self.has_seen_enough() {
            return None;
        }
        let floor = self.floor();
        let worth_now = self.worth_of_a_summary(self.context, price);
        let is_due = self.surplus >= worth_now;

        let (over, left) = if is_due {
            (0, self.worth_of_a_summary(floor, price))
        } else {
            let over = u128::from(self.context.saturating_sub(floor));
            (over, worth_now - self.surplus)
        };
        let growth = u128::from((self.growth / self.requests).max(SLOWEST_GROWTH));
        let climb = over
            .saturating_mul(over)
            .saturating_add(growth.saturating_mul(left).saturating_mul(2))
            .isqrt();
        let reach = u64::try_from(u128::from(floor).saturating_add(climb)).unwrap_or(u64::MAX);

        let window = if is_due {
            reach.min(self.context)
        } else {
            reach.saturating_add(SUMMARIZED_THIS_FAR_BELOW_THE_WINDOW)
        };
        Some(window.max(self.lowest_window()).min(largest))
    }

    const fn has_seen_enough(&self) -> bool {
        self.requests >= REQUESTS_BEFORE_A_WINDOW && !self.awaits_the_request_after_a_summary
    }

    fn floor(&self) -> u64 {
        self.summarized.map_or_else(
            || {
                self.base
                    .saturating_add(KEPT_BY_A_SUMMARY)
                    .saturating_add(READ_AGAIN_AFTER_A_SUMMARY)
            },
            |summarized| summarized.context,
        )
    }

    fn lowest_window(&self) -> u64 {
        let most_left = self.summarized.map_or_else(
            || self.base.saturating_add(MOST_KEPT_BY_A_SUMMARY),
            |summarized| summarized.context,
        );
        let roomy = u128::from(most_left) * u128::from(ROOM_OVER_A_SUMMARY_NUMERATOR)
            / u128::from(ROOM_OVER_A_SUMMARY_DENOMINATOR);
        u64::try_from(roomy)
            .unwrap_or(u64::MAX)
            .saturating_add(SUMMARIZED_THIS_FAR_BELOW_THE_WINDOW)
            .max(SMALLEST_WINDOW)
    }

    fn worth_of_a_summary(&self, context: u64, price: &Price) -> u128 {
        let rewritten = self.summarized.map_or_else(
            || {
                u128::from(self.floor()) * u128::from(REWRITTEN_AFTER_A_SUMMARY_PER_MILLE)
                    / u128::from(PER_MILLE)
            },
            |summarized| u128::from(summarized.written),
        );
        let write = if self.cache_lasts_five_minutes {
            price.cache_written_for_five_minutes
        } else {
            price.cache_written_for_an_hour
        };
        let read = u128::from(price.cache_read.max(1));

        (rewritten * u128::from(write) / read)
            .saturating_add(u128::from(SUMMARY_TOKENS) * u128::from(price.output) / read)
            .saturating_add(u128::from(context))
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-model/context.rs"]
mod tests;
