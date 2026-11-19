//! Benches of the interface's hot paths: `cargo bench -p dot-desktop --features bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dot_desktop::bench::{ConversationBench, ReplyBench, StripBench, cut_draft};

const HISTORY: usize = 100_000;
const LONG_REPLY: usize = 100 * 1_024;
const PIECE: &str = "a few more words ";
const DRAFT: usize = 1_000_000;
const DRAFT_COLUMNS: usize = 94;

fn frame_of_a_long_history(criterion: &mut Criterion) {
    let mut conversation = ConversationBench::with_history(HISTORY);
    criterion.bench_function("frame/history_100k", |bench| {
        bench.iter(|| black_box(conversation.frame()));
    });
}

fn piece_of_a_long_reply(criterion: &mut Criterion) {
    let mut reply = ReplyBench::of_length(LONG_REPLY);
    criterion.bench_function("reply/piece_in_100k", |bench| {
        bench.iter(|| reply.hear(black_box(PIECE)));
    });
}

fn strip_of_the_status_bar(criterion: &mut Criterion) {
    let mut strip = StripBench::new();
    criterion.bench_function("status_bar/strip", |bench| {
        bench.iter(|| black_box(strip.strip()));
    });
}

fn draft_of_a_million_characters(criterion: &mut Criterion) {
    let tabbed = "word\tword ".repeat(DRAFT / 10);
    criterion.bench_function("draft/cut_1m", |bench| {
        bench.iter(|| black_box(cut_draft(black_box(&tabbed), DRAFT_COLUMNS)));
    });
    let plain = "word word ".repeat(DRAFT / 10);
    criterion.bench_function("draft/cut_1m_without_tabs", |bench| {
        bench.iter(|| black_box(cut_draft(black_box(&plain), DRAFT_COLUMNS)));
    });
}

criterion_group!(
    interface,
    frame_of_a_long_history,
    piece_of_a_long_reply,
    strip_of_the_status_bar,
    draft_of_a_million_characters
);
criterion_main!(interface);
