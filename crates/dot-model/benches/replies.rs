//! Benches of the replies' hot paths: `cargo bench -p dot-model --features bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dot_model::bench::{
    claude_code_output, prepare_request, read_claude_code, read_claude_code_directly, read_reply,
    reply_stream,
};

const LONG_REPLY: usize = 100 * 1_024;
const HISTORY: usize = 1_024 * 1_024;

fn reading_a_long_reply(criterion: &mut Criterion) {
    let stream = reply_stream(LONG_REPLY);
    criterion.bench_function("stream/read_100k", |bench| {
        bench.iter(|| black_box(read_reply(black_box(&stream))));
    });
}

fn reading_claude_code(criterion: &mut Criterion) {
    let output: &'static [u8] = Box::leak(claude_code_output(LONG_REPLY).into_boxed_slice());
    criterion.bench_function("stream/claude_code_100k", |bench| {
        bench.iter(|| black_box(read_claude_code(black_box(output))));
    });
    criterion.bench_function("stream/claude_code_100k_directly", |bench| {
        bench.iter(|| black_box(read_claude_code_directly(black_box(output))));
    });
}

fn preparing_a_request(criterion: &mut Criterion) {
    let history = vec![b'x'; HISTORY];
    criterion.bench_function("request/prepare_1m_history", |bench| {
        bench.iter(|| black_box(prepare_request(black_box(&history))));
    });
}

criterion_group!(
    replies,
    reading_a_long_reply,
    reading_claude_code,
    preparing_a_request
);
criterion_main!(replies);
