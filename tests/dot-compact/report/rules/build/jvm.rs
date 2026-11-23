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
const TRANSFER: Option<&str> = Some(TRANSFER_PROGRESS);

#[test]
fn a_gradle_task_is_idle_unless_its_own_output_follows() {
    let lines = [
        "> Task :a UP-TO-DATE",
        "> Task :b",
        "",
        "> Task :c",
        "warning: something",
        "> Task :d",
        "",
        "BUILD SUCCESSFUL in 1s",
        "> Task :e",
    ];
    let idle = Some(IDLE_TASKS);

    assert_eq!(
        told("gradlew", &lines),
        [idle, idle, None, None, None, idle, None, None, idle]
    );
}

#[test]
fn what_gradle_says_while_it_sets_up_is_progress() {
    let lines = [
        "Starting a Gradle Daemon (subsequent builds will be faster)",
        "Reusing configuration cache.",
        "Calculating task graph as no cached configuration is available for tasks: build",
        "> Configure project :app",
        "Deprecated Gradle features were used in this build, making it incompatible with Gradle 9.0.",
        "You can use '--warning-mode all' to show the individual deprecation warnings.",
        "For more on this, please refer to https://docs.gradle.org/8.7/userguide/command_line_interface.html",
        "Downloading https://services.gradle.org/distributions/gradle-8.7-bin.zip",
        "BUILD SUCCESSFUL in 4s",
        "e: file:///src/App.kt:3:5 Unresolved reference: foo",
        "w: file:///src/App.kt:9:1 'bar()' is deprecated",
        "12 actionable tasks: 4 executed, 8 up-to-date",
    ];

    assert_eq!(
        told("gradle", &lines),
        [
            BUILD, BUILD, BUILD, BUILD, BUILD, BUILD, BUILD, BUILD, None, None, None, None
        ]
    );
}

#[test]
fn the_advice_after_a_failed_gradle_build_is_not_the_failure() {
    let lines = [
        "FAILURE: Build failed with an exception.",
        "* What went wrong:",
        "Execution failed for task ':app:compileKotlin'.",
        "* Try:",
        "> Run with --stacktrace option to get the stack trace.",
        "> Run with --info or --debug option to get more log output.",
        "> Get more help at https://help.gradle.org.",
        "BUILD FAILED in 3s",
    ];
    let advice = Some(ADVICE);

    assert_eq!(
        told("gradlew", &lines),
        [None, None, None, advice, advice, advice, advice, None]
    );
}

#[test]
fn maven_banners_blank_lines_and_downloads_are_progress() {
    let lines = [
        "[INFO] Scanning for projects...",
        "[INFO] ",
        "[INFO] ------------------------------------------------------------------------",
        "[INFO] --- maven-compiler-plugin:3.11.0:compile (default-compile) @ app ---",
        "[INFO] Building app 1.0",
        "[INFO] BUILD SUCCESS",
        "[WARNING] deprecated API",
        "[INFO] Total time:  2.1 s",
        "[INFO] Downloading from central: https://repo/a.pom",
        "Downloaded from central: https://repo/a.pom (2 kB at 10 kB/s)",
        "Progress (1): 4.1/12 kB",
        "[INFO] Reactor Summary for app 1.0:",
    ];

    assert_eq!(
        told("mvn", &lines),
        [
            BUILD, BUILD, BUILD, BUILD, None, None, None, None, TRANSFER, TRANSFER, TRANSFER, None
        ]
    );
}

#[test]
fn the_advice_after_a_failed_maven_build_is_not_the_failure() {
    let lines = [
        "[ERROR] COMPILATION ERROR : ",
        "[ERROR] /src/App.java:[3,5] cannot find symbol",
        "[ERROR] Failed to execute goal org.apache.maven.plugins:maven-compiler-plugin:3.11.0:compile",
        "[ERROR] -> [Help 1]",
        "[ERROR] ",
        "[ERROR] To see the full stack trace of the errors, re-run Maven with the -e switch.",
        "[ERROR] Re-run Maven using the -X switch to enable full debug logging.",
        "[ERROR] For more information about the errors and possible solutions, please read the following articles:",
        "[ERROR] [Help 1] http://cwiki.apache.org/confluence/display/MAVEN/MojoFailureException",
        "[INFO] BUILD FAILURE",
    ];
    let advice = Some(ADVICE);

    assert_eq!(
        told("mvnw", &lines),
        [
            None, None, None, advice, BUILD, advice, advice, advice, advice, None
        ]
    );
}

#[test]
fn the_frames_of_the_test_framework_are_not_the_ones_of_the_project() {
    let lines = [
        "java.lang.AssertionError: expected:<2> but was:<3>",
        "\tat org.junit.Assert.fail(Assert.java:89)",
        "    at java.base/jdk.internal.reflect.DirectMethodHandleAccessor.invoke(DirectMethodHandleAccessor.java:103)",
        "\tat org.gradle.api.internal.tasks.testing.junit.JUnitTestClassExecutor.execute(JUnitTestClassExecutor.java:55)",
        "\tat org.apache.maven.surefire.junit4.JUnit4Provider.execute(JUnit4Provider.java:316)",
        "\tat com.app.AppTest.adds(AppTest.java:12)",
        "at org.junit. is where JUnit lives",
    ];
    let frames = Some(FRAMEWORK_FRAMES);

    for tool in ["gradlew", "mvn", "sbt", "java"] {
        assert_eq!(
            told(tool, &lines),
            [None, frames, frames, frames, frames, None, frames],
            "{tool}"
        );
    }
}

#[test]
fn what_sbt_loads_and_compiles_is_progress() {
    let lines = [
        "[info] welcome to sbt 1.10.0 (Eclipse Adoptium Java 21)",
        "[info] loading settings for project app from build.sbt ...",
        "[info] compiling 3 Scala sources to /src/target/scala-3.4.2/classes ...",
        "[info] done compiling",
        "[success] Total time: 4 s, completed 19 nov. 2026, 18:00:00",
        "[error] /src/App.scala:3:5: not found: value foo",
        "[warn] one deprecation",
    ];

    assert_eq!(
        told("sbt", &lines),
        [BUILD, BUILD, BUILD, BUILD, None, None, None]
    );
}

#[test]
fn what_liquibase_says_before_it_starts_is_startup() {
    let lines = [
        "Starting Liquibase at 18:00:00 (version 4.27.0 #1525 built at 2024-03-25 17:08+0000)",
        "Liquibase Home: /opt/liquibase",
        "Libraries:",
        "- internal/lib/h2.jar",
        "  - lib/postgresql.jar",
        "- a change set",
        "Liquibase command 'update' was executed successfully.",
        "Running Changeset: db/changelog.xml::1::me",
    ];
    let startup = Some(STARTUP);

    assert_eq!(
        told("liquibase", &lines),
        [
            startup, startup, startup, startup, startup, None, None, None
        ]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told("gradle", &["[INFO] ", "[info] done compiling"]),
        [None, None]
    );
    assert_eq!(
        told("mvn", &["> Task :a UP-TO-DATE", "* Try:"]),
        [None, None]
    );
}
