//! Benches of the interface's hot paths: `cargo bench -p dot-desktop --features bench`.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use dot_desktop::bench::{
    CommandBench, ConversationBench, ReplyBench, StripBench, TintBench, ToolBench, cut_draft,
};

const HISTORY: usize = 100_000;
const HISTORY_WITH_TOOLS: usize = 10_000;
const TOOLS_PER_REPLY: usize = 5;
const TOOLS_ALREADY_RUN: usize = 20;
const COMMANDS_PER_REPLY: usize = 5;
const AGENTS: usize = 3;
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

fn frames_with_commands(criterion: &mut Criterion) {
    let mut conversation = ConversationBench::with_commands(HISTORY_WITH_TOOLS, COMMANDS_PER_REPLY);
    criterion.bench_function("frame/commands_10k", |bench| {
        bench.iter(|| black_box(conversation.frame()));
    });
    let mut running = ConversationBench::with_command_running(TOOLS_ALREADY_RUN);
    criterion.bench_function("frame/command_running", |bench| {
        bench.iter(|| black_box(running.frame()));
    });
}

fn frames_with_agents(criterion: &mut Criterion) {
    let mut working = ConversationBench::with_agents_working(AGENTS);
    criterion.bench_function("frame/agents_working", |bench| {
        bench.iter(|| black_box(working.frame()));
    });
    let mut out_of_sight = ConversationBench::with_agents_out_of_sight(AGENTS);
    criterion.bench_function("frame/agents_out_of_sight", |bench| {
        bench.iter(|| black_box(out_of_sight.frame()));
    });
}

fn frames_with_a_selection(criterion: &mut Criterion) {
    let mut set = ConversationBench::with_selection();
    criterion.bench_function("frame/selection_set", |bench| {
        bench.iter(|| black_box(set.frame()));
    });
    let mut dragged = ConversationBench::while_selecting();
    criterion.bench_function("frame/selection_dragged", |bench| {
        bench.iter(|| black_box(dragged.frame()));
    });
}

fn command_of_a_reply(criterion: &mut Criterion) {
    criterion.bench_function("reply/command_after_20", |bench| {
        bench.iter_batched_ref(
            || CommandBench::after(TOOLS_ALREADY_RUN),
            CommandBench::run_command,
            BatchSize::SmallInput,
        );
    });
}

fn piece_of_a_long_reply(criterion: &mut Criterion) {
    let mut reply = ReplyBench::of_length(LONG_REPLY);
    criterion.bench_function("reply/piece_in_100k", |bench| {
        bench.iter(|| reply.hear(black_box(PIECE)));
    });
    let mut paced = ReplyBench::paced_of_length(LONG_REPLY);
    criterion.bench_function("reply/paced_piece_in_100k", |bench| {
        bench.iter(|| paced.hear_a_frame_later(black_box(PIECE)));
    });
}

fn strip_of_the_status_bar(criterion: &mut Criterion) {
    let mut strip = StripBench::new();
    criterion.bench_function("status_bar/strip", |bench| {
        bench.iter(|| black_box(strip.strip()));
    });
}

fn colours_of_meaning(criterion: &mut Criterion) {
    let frame = TintBench::with_tools(HISTORY_WITH_TOOLS, TOOLS_PER_REPLY);
    criterion.bench_function("tint/frame_of_tools", |bench| {
        bench.iter(|| black_box(frame.tint()));
    });
    criterion.bench_function("tint/copy_only", |bench| {
        bench.iter(|| black_box(frame.copy()));
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
    frames_with_commands,
    frames_with_agents,
    frames_with_a_selection,
    piece_of_a_long_reply,
    tool_of_a_reply,
    command_of_a_reply,
    strip_of_the_status_bar,
    colours_of_meaning,
    draft_of_a_million_characters
);
criterion_main!(interface);
