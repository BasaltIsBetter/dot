use crate::report::rules::{BUILD_PROGRESS, Lines, Rule, after_counter, starts_with_any};

const INCLUDE_CHAIN: &str = "lines: include chain";

const C_TOOLS: [&str; 10] = [
    "c++", "cc", "clang", "clang++", "cmake", "g++", "gcc", "gmake", "make", "ninja",
];
const MAKE: [&str; 3] = ["make[", "make: ", "gmake["];
const MAKE_FOLDERS: [&str; 2] = [": Entering directory", ": Leaving directory"];
const CMAKE_CHECK: &str = "-- ";
const CMAKE_RESULTS: [&str; 4] = [
    "-- Build files have been written",
    "-- Configuring done",
    "-- Configuring incomplete",
    "-- Generating done",
];
const COUNTED_STEPS: [&str; 8] = [
    "Building ",
    "Compiling ",
    "Emitting ",
    "Generating ",
    "Linking ",
    "Scanning dependencies",
    "Write ",
    "Wrapping ",
];
const INCLUDED_FROM: &str = "In file included from ";
const INCLUDED_NEXT: &str = "from ";
const SWIFT_STEPS: [&str; 10] = [
    "Building for ",
    "Cloning ",
    "Compiling ",
    "Computed ",
    "Computing version for ",
    "Creating working copy for ",
    "Fetched ",
    "Fetching ",
    "Linking ",
    "Working copy of ",
];
const XCODE_STEPS: [&str; 26] = [
    "CodeSign ",
    "CompileAssetCatalog",
    "CompileC ",
    "CompileStoryboard",
    "CompileSwift ",
    "CopySwiftLibs ",
    "CpHeader ",
    "CpResource ",
    "CreateBuildDirectory ",
    "Ditto ",
    "GenerateDSYMFile",
    "Ld ",
    "LinkStoryboards",
    "MkDir ",
    "PBXCp ",
    "PhaseScriptExecution",
    "ProcessInfoPlistFile ",
    "ProcessProductPackaging",
    "RegisterWithLaunchServices",
    "SetMode ",
    "SetOwnerAndGroup ",
    "Signing Identity:",
    "SwiftDriver",
    "Touch ",
    "Validate ",
    "note: Using new build system",
];
const XCODE_COMMANDS: [&str; 5] = [
    "/Applications/Xcode",
    "/usr/bin/",
    "builtin-",
    "cd /",
    "export ",
];
const EMBEDDED_STEPS: [&str; 6] = [
    "Archiving ",
    "Building ",
    "Checking size",
    "Compiling ",
    "Indexing ",
    "Linking ",
];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["make", "gmake"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(make_folder),
    },
    Rule {
        tools: &["cmake"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(cmake_check),
    },
    Rule {
        tools: &["cmake", "make", "gmake", "ninja", "meson", "swift"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(counted_step),
    },
    Rule {
        tools: &C_TOOLS,
        what: INCLUDE_CHAIN,
        lines: Lines::Told(include_chain),
    },
    Rule {
        tools: &["swift"],
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&SWIFT_STEPS),
    },
    Rule {
        tools: &["xcodebuild"],
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&XCODE_STEPS),
    },
    Rule {
        tools: &["xcodebuild"],
        what: BUILD_PROGRESS,
        lines: Lines::Told(xcode_command),
    },
    Rule {
        tools: &["pio", "platformio"],
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&EMBEDDED_STEPS),
    },
];

fn make_folder(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    starts_with_any(line, &MAKE) && MAKE_FOLDERS.iter().any(|folder| line.contains(folder))
}

fn cmake_check(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line.starts_with(CMAKE_CHECK) && !starts_with_any(line, &CMAKE_RESULTS)
}

fn counted_step(lines: &[&str], at: usize) -> bool {
    after_counter(lines[at], '[', ']').is_some_and(|step| starts_with_any(step, &COUNTED_STEPS))
}

fn include_chain(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let continues = line.starts_with(' ') && line.trim_start().starts_with(INCLUDED_NEXT);
    line.starts_with(INCLUDED_FROM) || continues
}

fn xcode_command(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    line.starts_with(' ') && starts_with_any(line.trim_start(), &XCODE_COMMANDS)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/build/native.rs"]
mod tests;
