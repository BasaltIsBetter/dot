use crate::report::rules::{BUILD_PROGRESS, Lines, Rule, TRANSFER_PROGRESS, starts_with_any};

const IDLE_TASKS: &str = "lines: tasks with nothing to report";
const ADVICE: &str = "lines: advice";
const FRAMEWORK_FRAMES: &str = "lines: test framework frames";
const STARTUP: &str = "lines: startup messages";

const GRADLE: [&str; 2] = ["gradle", "gradlew"];
const MAVEN: [&str; 2] = ["mvn", "mvnw"];
const GRADLE_TASK: &str = "> Task :";
const GRADLE_END: &str = "BUILD ";
const GRADLE_SETUP: [&str; 16] = [
    "> Configure project",
    "Calculating task graph",
    "Configuration cache entry",
    "Configuring ",
    "Daemon will be stopped",
    "Deprecated Gradle features",
    "Download https://",
    "Downloading ",
    "For more on this",
    "Resolving ",
    "Reusing configuration cache",
    "Starting a Gradle Daemon",
    "To honour the JVM settings",
    "You can use ",
    "[Incubating]",
    "[android-",
];
const GRADLE_ADVICE: [&str; 3] = ["* Try:", "> Get more help at", "> Run with --"];
const MAVEN_TRANSFER: [&str; 6] = [
    "Downloaded from ",
    "Downloading from ",
    "Progress (",
    "[INFO] Downloaded from ",
    "[INFO] Downloading from ",
    "[INFO] Download",
];
const MAVEN_BLANKS: [&str; 2] = ["[INFO]", "[ERROR]"];
const MAVEN_BANNERS: [&str; 2] = ["[INFO] ---", "[INFO] Scanning for projects"];
const MAVEN_ADVICE: [&str; 5] = [
    "[ERROR] -> [Help ",
    "[ERROR] For more information about the errors",
    "[ERROR] Re-run Maven using",
    "[ERROR] To see the full stack trace",
    "[ERROR] [Help ",
];
const FRAMES: [&str; 11] = [
    "at java.base/java.lang.reflect.",
    "at java.base/jdk.internal.reflect.",
    "at java.lang.reflect.",
    "at jdk.internal.reflect.",
    "at junit.",
    "at org.apache.maven.surefire.",
    "at org.gradle.",
    "at org.junit.",
    "at org.scalatest.",
    "at sbt.",
    "at sun.reflect.",
];
const SBT_STEPS: [&str; 12] = [
    "[info] Compiling ",
    "[info] Fetched ",
    "[info] Fetching ",
    "[info] Resolved ",
    "[info] Updating ",
    "[info] compiling ",
    "[info] done compiling",
    "[info] downloading ",
    "[info] loading ",
    "[info] resolving ",
    "[info] set current project",
    "[info] welcome to sbt",
];
const LIQUIBASE_STARTUP: [&str; 8] = [
    "INFO [liquibase.core] Parsing",
    "INFO [liquibase.core] Reading resource",
    "INFO [liquibase.integration]",
    "Java Home",
    "Libraries:",
    "Liquibase Home:",
    "Starting Liquibase at",
    "##",
];
const JAR: &str = ".jar";
const LISTED: &str = "- ";

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &GRADLE,
        what: IDLE_TASKS,
        lines: Lines::Told(gradle_idle_task),
    },
    Rule {
        tools: &GRADLE,
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&GRADLE_SETUP),
    },
    Rule {
        tools: &GRADLE,
        what: ADVICE,
        lines: Lines::Starting(&GRADLE_ADVICE),
    },
    Rule {
        tools: &MAVEN,
        what: BUILD_PROGRESS,
        lines: Lines::Told(maven_banner),
    },
    Rule {
        tools: &MAVEN,
        what: TRANSFER_PROGRESS,
        lines: Lines::Starting(&MAVEN_TRANSFER),
    },
    Rule {
        tools: &MAVEN,
        what: ADVICE,
        lines: Lines::Starting(&MAVEN_ADVICE),
    },
    Rule {
        tools: &["gradle", "gradlew", "java", "mvn", "mvnw", "sbt"],
        what: FRAMEWORK_FRAMES,
        lines: Lines::Indented(&FRAMES),
    },
    Rule {
        tools: &["sbt"],
        what: BUILD_PROGRESS,
        lines: Lines::Starting(&SBT_STEPS),
    },
    Rule {
        tools: &["liquibase"],
        what: STARTUP,
        lines: Lines::Told(liquibase_startup),
    },
];

fn gradle_idle_task(lines: &[&str], at: usize) -> bool {
    if !lines[at].starts_with(GRADLE_TASK) {
        return false;
    }
    lines[at + 1..]
        .iter()
        .find(|line| !line.trim().is_empty())
        .is_none_or(|next| next.starts_with(GRADLE_TASK) || next.starts_with(GRADLE_END))
}

fn maven_banner(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_end();
    MAVEN_BLANKS.contains(&line) || starts_with_any(line, &MAVEN_BANNERS)
}

fn liquibase_startup(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let library = line.trim_start().starts_with(LISTED) && line.trim_end().ends_with(JAR);
    library || starts_with_any(line, &LIQUIBASE_STARTUP)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/build/jvm.rs"]
mod tests;
