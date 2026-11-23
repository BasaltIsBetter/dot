use super::*;

fn told(tool: &str, lines: &[&str]) -> Vec<Option<&'static str>> {
    (0..lines.len())
        .map(|at| {
            RULES
                .iter()
                .filter(|rule| rule.tools.contains(&tool))
                .find(|rule| rule.drops(lines, at))
                .map(|rule| rule.what)
        })
        .collect()
}

const BUILD: Option<&str> = Some(BUILD_PROGRESS);

#[test]
fn a_cargo_step_is_indented_and_a_benchmark_says_it_is_running() {
    let lines = [
        "   Compiling a v1.0.0",
        "Compiling a list",
        "    Finished in 1s",
        "     Running unittests src/lib.rs",
        "Benchmarking search: Warming up for 3.0000 s",
        "search    time:   [1.2 µs 1.3 µs 1.4 µs]",
    ];

    assert_eq!(
        told("cargo", &lines),
        [BUILD, None, None, None, Some(BENCHMARK_PROGRESS), None]
    );
    assert_eq!(told("trunk", &lines), [BUILD, None, None, None, None, None]);
    assert_eq!(told("cross", &lines), [BUILD, None, None, None, None, None]);
}

#[test]
fn a_dotnet_restore_and_its_banner_are_progress_and_what_it_built_is_not() {
    let lines = [
        "Microsoft (R) Build Engine version 17.8.3+195e7f5a3 for .NET",
        "Copyright (C) Microsoft Corporation. All rights reserved.",
        "  Determining projects to restore...",
        "  Restored /src/App.csproj (in 312 ms).",
        "  All projects are up-to-date for restore.",
        "  App -> /src/bin/Debug/net8.0/App.dll",
        "Build succeeded.",
        "    0 Warning(s)",
        "/src/App.cs(3,5): error CS0103: The name 'foo' does not exist",
    ];

    assert_eq!(
        told("dotnet", &lines),
        [BUILD, BUILD, BUILD, BUILD, BUILD, None, None, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "go",
            &["   Compiling a v1.0.0", "  Restored /src/App.csproj"]
        ),
        [None, None]
    );
}

#[test]
fn the_steps_of_a_next_build_are_progress_and_its_routes_are_not() {
    let lines = [
        "   ▲ Next.js 15.2.0",
        "   Creating an optimized production build ...",
        " ✓ Compiled successfully",
        "   Linting and checking validity of types ...",
        "   Collecting page data ...",
        "   Generating static pages (0/12) ...",
        " ✓ Generating static pages (12/12)",
        "   Finalizing page optimization ...",
        "   Collecting build traces ...",
        "Route (app)                              Size     First Load JS",
        "┌ ○ /                                    5.2 kB         92 kB",
        " ✓ Ready in 1.2s",
        "Failed to compile.",
    ];
    let step = Some(BUILD_PROGRESS);

    assert_eq!(
        told("next", &lines),
        [
            None, step, step, step, step, step, step, step, step, None, None, None, None
        ]
    );
}

#[test]
fn what_mix_compiles_and_fetches() {
    let lines = [
        "==> phoenix",
        "Compiling 71 files (.ex)",
        "Generated phoenix app",
        "* Getting jason (Hex package)",
        "  Checking package (https://repo.hex.pm/tarballs/jason-1.4.1.tar)",
        "  Fetched package",
        "  Using locally cached package",
        "warning: variable \"conn\" is unused",
        "  lib/router.ex:42",
        "  Compiling in a sentence",
        "Unchanged:",
    ];
    let built = Some(BUILD_PROGRESS);
    let fetched = Some(INSTALL_PROGRESS);

    assert_eq!(
        told("mix", &lines),
        [
            built, built, built, fetched, fetched, fetched, fetched, None, None, None, None
        ]
    );
    assert_eq!(told("dotnet", &lines), [None; 11]);
}
