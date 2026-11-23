use dot_api::Cost;

use crate::stream::Usage;

const NANO_DOLLARS_PER_MICRO_DOLLAR: u64 = 1_000;
const OPUS_5_PRICED_MODELS: [&str; 2] = ["claude-opus-5", "claude-opus-4-8"];
const OPUS_5_5: Price = Price {
    input: 4_000,
    output: 20_000,
    cache_written_for_five_minutes: 5_000,
    cache_written_for_an_hour: 8_000,
    cache_read: 200,
};
const OPUS_5: Price = Price {
    input: 5_000,
    output: 25_000,
    cache_written_for_five_minutes: 6_250,
    cache_written_for_an_hour: 10_000,
    cache_read: 500,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Price {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) cache_written_for_five_minutes: u64,
    pub(crate) cache_written_for_an_hour: u64,
    pub(crate) cache_read: u64,
}

pub(crate) fn cost(model: &str, usage: &Usage) -> Cost {
    let price = price_of(model);
    let written_for_an_hour = usage.cache_written_for_an_hour();
    let written_for_five_minutes = usage.cache_written() - written_for_an_hour;

    let nano_dollars = [
        (usage.input(), price.input),
        (usage.output(), price.output),
        (
            written_for_five_minutes,
            price.cache_written_for_five_minutes,
        ),
        (written_for_an_hour, price.cache_written_for_an_hour),
        (usage.cache_read(), price.cache_read),
    ]
    .into_iter()
    .fold(0_u64, |sum, (tokens, price)| {
        sum.saturating_add(tokens.saturating_mul(price))
    });

    let rounded = nano_dollars.saturating_add(NANO_DOLLARS_PER_MICRO_DOLLAR / 2);
    Cost::from_micro_dollars(rounded / NANO_DOLLARS_PER_MICRO_DOLLAR)
}

pub(crate) fn billed_since(billed: &mut u64, total: Cost) -> Cost {
    let total = total.micro_dollars();
    let before = std::mem::replace(billed, total);
    Cost::from_micro_dollars(total.checked_sub(before).unwrap_or(total))
}

pub(crate) fn price_of(model: &str) -> Price {
    if OPUS_5_PRICED_MODELS.contains(&model) {
        return OPUS_5;
    }
    OPUS_5_5
}

#[cfg(test)]
#[path = "../../../tests/dot-model/pricing.rs"]
mod tests;
