//! Benches of the interface's hot paths: `cargo bench -p dot-desktop --features bench`.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use dot_desktop::bench::{ConversationBench, ReplyBench, StripBench, ToolBench, cut_draft};

const HISTORY: usize = 100_000;
const HISTORY_WITH_TOOLS: usize = 10_000;
const TOOLS_PER_REPLY: usize = 5;
const TOOLS_ALREADY_RUN: usize = 20;
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

fn frames_with_tools(criterion: &mut Criterion) {
    let mut conversation = ConversationBench::with_tools(HISTORY_WITH_TOOLS, TOOLS_PER_REPLY);
    criterion.bench_function("frame/tools_10k", |bench| {
        bench.iter(|| black_box(conversation.frame()));
    });
    let mut arriving = ConversationBench::with_tool_arriving(TOOLS_ALREADY_RUN);
    criterion.bench_function("frame/tool_arriving", |bench| {
        bench.iter(|| black_box(arriving.frame()));
    });
}

fn tool_of_a_reply(criterion: &mut Criterion) {
    criterion.bench_function("reply/tool_after_20", |bench| {
        bench.iter_batched_ref(
            || ToolBench::after(TOOLS_ALREADY_RUN),
            ToolBench::run_tool,
            BatchSize::SmallInput,
        );
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
    frames_with_tools,
    piece_of_a_long_reply,
    tool_of_a_reply,
    strip_of_the_status_bar,
    draft_of_a_million_characters
);
criterion_main!(interface);
