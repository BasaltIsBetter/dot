//! Benches of the replies' hot paths: `cargo bench -p dot-model --features bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dot_model::bench::{prepare_request, read_reply, reply_stream};

const LONG_REPLY: usize = 100 * 1_024;
const HISTORY: usize = 1_024 * 1_024;

fn reading_a_long_reply(criterion: &mut Criterion) {
    let stream = reply_stream(LONG_REPLY);
    criterion.bench_function("stream/read_100k", |bench| {
        bench.iter(|| black_box(read_reply(black_box(&stream))));
    });
}

fn preparing_a_request(criterion: &mut Criterion) {
    let history = vec![b'x'; HISTORY];
    criterion.bench_function("request/prepare_1m_history", |bench| {
        bench.iter(|| black_box(prepare_request(black_box(&history))));
    });
}

criterion_group!(replies, reading_a_long_reply, preparing_a_request);
criterion_main!(replies);
