//! Benchmarks of the compaction of tool outputs, the largest an output gets before Claude Code
//! sets it aside (30,000 characters), and of an output that lasts, read the way `dot-shell`'s
//! relay reads it.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dot_api::{Compactor, Origin};
use dot_compact::ShapeCompactor;

const LARGEST_OUTPUT: usize = 30_000;
const PIECE: usize = 1_024;
const HELD_BY_THE_RELAY: usize = 1_024 * 1_024;
const READ_BY_THE_RELAY: usize = 64 * 1_024;
const LARGEST_LASTING_OUTPUT: usize = 2 * HELD_BY_THE_RELAY;

fn filled(length: usize, mut line: impl FnMut(usize) -> String) -> String {
    let mut output = String::with_capacity(length);
    let mut number = 0;
    while output.len() < length {
        output.push_str(&line(number));
        number += 1;
    }
    output
}

fn json_items(length: usize) -> String {
    let items = filled(length, |number| {
        format!("{{\"id\":{number},\"name\":\"item {number}\"}},")
    });
    format!("{{\"items\":[{}]}}\n", items.trim_end_matches(','))
}

fn as_the_relay_reads(output: &str) -> Vec<&str> {
    let (held, rest) = output.split_at(HELD_BY_THE_RELAY.min(output.len()));
    let read = (0..rest.len())
        .step_by(READ_BY_THE_RELAY)
        .map(|at| &rest[at..rest.len().min(at + READ_BY_THE_RELAY)]);
    std::iter::once(held).chain(read).collect()
}

fn lasting(command: &str, pieces: &[&str]) -> usize {
    let Some(mut stream) = ShapeCompactor::default().stream(command) else {
        return 0;
    };
    let Some(mut burst) = ShapeCompactor::default().burst(command) else {
        return 0;
    };
    let told: usize = pieces
        .iter()
        .map(|piece| burst.pass(stream.push(piece).as_bytes()).len())
        .sum();
    told + burst.pass(stream.rest().as_bytes()).len() + burst.pause().len()
}

fn compaction(criterion: &mut Criterion) {
    let matches = filled(LARGEST_OUTPUT, |number| {
        format!(
            "crates/dot-model/src/file_{}.rs:{number}:    let reply = chat.reply(prompt);\n",
            number / 9
        )
    });
    let paths = filled(LARGEST_OUTPUT, |number| {
        format!(
            "./crates/dot-desktop/src/folder_{}/file_{number}.rs\n",
            number / 7
        )
    });
    let tests = filled(LARGEST_OUTPUT, |number| {
        format!("test conversation::flow::tests::case_{number} ... ok\n")
    });
    let prose = filled(LARGEST_OUTPUT, |number| {
        format!("line {number} of an output no rule recognises\n")
    });
    let compact_json = json_items(LARGEST_OUTPUT);
    let long_prose = filled(LARGEST_LASTING_OUTPUT, |number| {
        format!("line {number} of an output no rule recognises\n")
    });

    criterion.bench_function("search results of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default().compact(
                Origin::Command("grep -rn reply crates"),
                black_box(&matches),
            )
        });
    });
    criterion.bench_function("file list of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default()
                .compact(Origin::Command("find . -name '*.rs'"), black_box(&paths))
        });
    });
    criterion.bench_function("test run of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default().compact(Origin::Command("cargo test"), black_box(&tests))
        });
    });
    criterion.bench_function("unrecognised output of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default()
                .compact(Origin::Command("python3 report.py"), black_box(&prose))
        });
    });
    criterion.bench_function("file contents of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default().compact(Origin::Command("cat src/lib.rs"), black_box(&prose))
        });
    });
    criterion.bench_function("compact JSON of 30 KB", |bencher| {
        bencher.iter(|| {
            ShapeCompactor::default().compact(
                Origin::Command("curl -s https://example.com/items"),
                black_box(&compact_json),
            )
        });
    });
    criterion.bench_function("lasting output of 30 KB, read 1 KB at a time", |bencher| {
        let pieces: Vec<&str> = (0..prose.len())
            .step_by(PIECE)
            .map(|at| &prose[at..prose.len().min(at + PIECE)])
            .collect();
        bencher.iter(|| lasting("python3 report.py", black_box(&pieces)));
    });
    criterion.bench_function(
        "lasting output of 2 MiB, read as the relay reads it",
        |bencher| {
            let pieces = as_the_relay_reads(&long_prose);
            bencher.iter(|| lasting("python3 report.py", black_box(&pieces)));
        },
    );
}

criterion_group!(benches, compaction);
criterion_main!(benches);
