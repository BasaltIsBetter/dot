use dot_api::{Cost, Model};

use crate::stream::Usage;

const NANO_DOLLARS_PER_MICRO_DOLLAR: u64 = 1_000;
const OPUS_5_PRICED_MODELS: [&str; 2] = ["claude-opus-5", "claude-opus-4-8"];
const HAIKU_LONG_PROMPT_FROM: u64 = 100_000;
const HAIKU_5_5: Price = Price {
    input: 100,
    output: 500,
    cache_written_for_five_minutes: 125,
    cache_written_for_an_hour: 200,
    cache_read: 10,
};
const HAIKU_5_5_LONG_PROMPT: Price = Price {
    input: 500,
    output: 2_500,
    cache_written_for_five_minutes: 625,
    cache_written_for_an_hour: 1_000,
    cache_read: 50,
};
const SONNET_5_5: Price = Price {
    input: 2_000,
    output: 10_000,
    cache_written_for_five_minutes: 2_500,
    cache_written_for_an_hour: 4_000,
    cache_read: 100,
};
const OPUS_5_5: Price = Price {
    input: 4_000,
    output: 20_000,
    cache_written_for_five_minutes: 5_000,
    cache_written_for_an_hour: 8_000,
    cache_read: 200,
};
const FABLE_5_2: Price = Price {
    input: 8_000,
    output: 40_000,
    cache_written_for_five_minutes: 10_000,
    cache_written_for_an_hour: 16_000,
    cache_read: 200,
};
const OPUS_5: Price = Price {
    input: 5_000,
    output: 25_000,
    cache_written_for_five_minutes: 6_250,
    cache_written_for_an_hour: 10_000,
    cache_read: 500,
};
const PRICES: [(&str, Price); 4] = [
    (Model::HAIKU_5_5.id, HAIKU_5_5),
    (Model::SONNET_5_5.id, SONNET_5_5),
    (Model::OPUS_5_5.id, OPUS_5_5),
    (Model::FABLE_5_2.id, FABLE_5_2),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Price {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) cache_written_for_five_minutes: u64,
    pub(crate) cache_written_for_an_hour: u64,
    pub(crate) cache_read: u64,
}

pub(crate) fn cost(model: &str, usage: &Usage) -> Cost {
    let price = price_for(model, usage);
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
    PRICES
        .iter()
        .find(|(id, _)| *id == model)
        .map_or(OPUS_5_5, |(_, price)| *price)
}

fn price_for(model: &str, usage: &Usage) -> Price {
    let prompt = usage
        .input()
        .saturating_add(usage.cache_read())
        .saturating_add(usage.cache_written());
    if model == Model::HAIKU_5_5.id && prompt > HAIKU_LONG_PROMPT_FROM {
        return HAIKU_5_5_LONG_PROMPT;
    }
    price_of(model)
}

#[cfg(test)]
#[path = "../../../tests/dot-model/pricing.rs"]
mod tests;
