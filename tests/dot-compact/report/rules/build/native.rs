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
fn make_entering_and_leaving_a_folder_is_progress() {
    let lines = [
        "make[1]: Entering directory '/src/lib'",
        "make[1]: Leaving directory '/src/lib'",
        "make: Entering directory '/src'",
        "gmake[2]: Leaving directory '/src'",
        "make: Nothing to be done for 'all'.",
        "make[1]: *** [Makefile:12: app] Error 1",
        "cc -c a.c -o a.o",
        "note: Entering directory",
    ];

    assert_eq!(
        told("make", &lines),
        [BUILD, BUILD, BUILD, BUILD, None, None, None, None]
    );
}

#[test]
fn what_cmake_checks_is_progress_and_what_it_concludes_is_not() {
    let lines = [
        "-- The C compiler identification is AppleClang 16.0.0",
        "-- Looking for pthread.h - found",
        "-- Configuring done (0.4s)",
        "-- Generating done (0.0s)",
        "-- Build files have been written to: /src/build",
        "-- Configuring incomplete, errors occurred!",
        "CMake Warning at CMakeLists.txt:3:",
    ];

    assert_eq!(
        told("cmake", &lines),
        [BUILD, BUILD, None, None, None, None, None]
    );
}

#[test]
fn a_counted_step_is_progress_and_a_built_target_is_not() {
    let lines = [
        "[ 12%] Building C object CMakeFiles/app.dir/a.c.o",
        "[100%] Linking C executable app",
        "[100%] Built target app",
        "[12/340] Compiling App Foo.swift",
        "[3/4] Running tests",
        "[note] Building",
    ];

    for tool in ["cmake", "make", "ninja", "meson", "swift"] {
        assert_eq!(
            told(tool, &lines),
            [BUILD, BUILD, None, BUILD, None, None],
            "{tool}"
        );
    }
}

#[test]
fn the_chain_of_includes_above_a_diagnostic_is_not_the_diagnostic() {
    let lines = [
        "In file included from src/a.c:3:",
        "                 from src/b.c:1:",
        "src/a.h:12:5: error: unknown type name 'foo'",
        "from the start of a sentence",
        "   12 |     foo x;",
    ];
    let chain = Some(INCLUDE_CHAIN);

    for tool in ["gcc", "g++", "clang", "make", "ninja"] {
        assert_eq!(
            told(tool, &lines)[..4],
            [chain, chain, None, None],
            "{tool}"
        );
    }
}

#[test]
fn what_swift_fetches_and_compiles_is_progress() {
    let lines = [
        "Fetching https://github.com/apple/swift-log",
        "Computing version for https://github.com/apple/swift-log",
        "Building for debugging...",
        "Compiling App Foo.swift",
        "Linking App",
        "Build complete! (3.21s)",
        "warning: unused variable",
        "/src/App.swift:3:5: error: cannot find 'foo' in scope",
    ];

    assert_eq!(
        told("swift", &lines),
        [BUILD, BUILD, BUILD, BUILD, BUILD, None, None, None]
    );
}

#[test]
fn the_steps_of_xcodebuild_and_the_commands_under_them_are_progress() {
    let lines = [
        "CompileSwift normal arm64 /src/App.swift (in target 'App' from project 'App')",
        "    cd /src",
        "    export LANG=en_US.US-ASCII",
        "    /Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/swiftc -c",
        "    builtin-copy -exclude .DS_Store",
        "CodeSign /src/build/App.app (in target 'App' from project 'App')",
        "note: Using new build system",
        "** BUILD SUCCEEDED **",
        "/src/App.swift:3:5: error: cannot find 'foo' in scope",
        "cd /src",
        "    return foo",
    ];

    assert_eq!(
        told("xcodebuild", &lines),
        [
            BUILD, BUILD, BUILD, BUILD, BUILD, BUILD, BUILD, None, None, None, None
        ]
    );
}

#[test]
fn what_platformio_compiles_is_progress() {
    let lines = [
        "Compiling .pio/build/esp32/src/main.cpp.o",
        "Linking .pio/build/esp32/firmware.elf",
        "Checking size .pio/build/esp32/firmware.elf",
        "RAM:   [=         ]   6.3% (used 20672 bytes from 327680 bytes)",
        "src/main.cpp:3:5: error: 'foo' was not declared in this scope",
    ];

    assert_eq!(told("pio", &lines), [BUILD, BUILD, BUILD, None, None]);
    assert_eq!(
        told("platformio", &lines),
        [BUILD, BUILD, BUILD, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "cmake",
            &[
                "make[1]: Entering directory '/src'",
                "CompileSwift normal arm64"
            ]
        ),
        [None, None]
    );
    assert_eq!(told("swift", &["-- Looking for pthread.h - found"]), [None]);
}
