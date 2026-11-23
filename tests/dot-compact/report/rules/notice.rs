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

#[test]
fn the_files_prettier_left_unchanged() {
    let lines = [
        "src/app.ts 45ms (unchanged)",
        "src/lib.ts 12ms",
        "[warn] src/old.ts",
        "(unchanged) said first",
    ];

    assert_eq!(
        told("prettier", &lines),
        [Some(UNCHANGED_FILES), None, None, None]
    );
}

#[test]
fn the_links_shellcheck_adds_under_its_findings() {
    let lines = [
        "In probe.sh line 2:",
        "echo $foo",
        "     ^--^ SC2086 (info): Double quote to prevent globbing and word splitting.",
        "For more information:",
        "  https://www.shellcheck.net/wiki/SC2045 -- Iterating over ls output is fragi...",
        "  https://www.shellcheck.net/wiki/SC2154 -- dir is referenced but not assigned.",
        "For more information: read the manual",
        "see https://www.shellcheck.net/wiki/SC2154",
    ];
    let footer = Some(RULE_LINKS);

    assert_eq!(
        told("shellcheck", &lines),
        [None, None, None, footer, footer, footer, None, None]
    );
}

#[test]
fn the_hints_and_the_update_box_of_prisma() {
    let lines = [
        "Prisma schema loaded from prisma/schema.prisma",
        "✔ Generated Prisma Client (v5.22.0) to ./node_modules/@prisma/client in 123ms",
        "Start by importing your Prisma Client (See: https://pris.ly/d/importing-client)",
        "Tip: Want to turn off tips and other hints? https://pris.ly/tip-4-nohints",
        "┌─────────────────────────────────────────────────────────┐",
        "│  Update available 5.22.0 -> 6.1.0                       │",
        "└─────────────────────────────────────────────────────────┘",
        "  └─ 20240101000000_init/",
        "A Tip: in a sentence",
    ];
    let notice = Some(NOTICES);

    assert_eq!(
        told("prisma", &lines),
        [
            None, None, notice, notice, notice, notice, notice, None, None
        ]
    );
}

#[test]
fn the_legend_under_a_list_of_systemd_units() {
    let lines = [
        "  nginx.service loaded active running A high performance web server",
        "LOAD   = Reflects whether the unit definition was properly loaded.",
        "ACTIVE = The high-level unit activation state, i.e. generalization of SUB.",
        "SUB    = The low-level unit activation state, values depend on unit type.",
        "Legend: LOAD   → Reflects whether the unit definition was properly loaded.",
        "        ACTIVE → The high-level unit activation state, i.e. generalization of SUB.",
        "        SUB    → The low-level unit activation state, values depend on unit type.",
        "176 loaded units listed.",
        "LOADED units are listed",
        "Legend: nothing of the kind",
        "ACTIVE units: 12",
    ];
    let legend = Some(LEGEND);

    assert_eq!(
        told("systemctl", &lines),
        [
            None, legend, legend, legend, legend, legend, legend, None, None, None, None
        ]
    );
}

#[test]
fn what_gcloud_adds_about_itself() {
    let lines = [
        "NAME        REGION        STATUS",
        "Updates are available for some Google Cloud CLI components.  To install them,",
        "please run:",
        "  $ gcloud components update",
        "To take a quick anonymous survey, run:",
        "  $ gcloud survey",
        "  $ gcloud compute instances list",
        "Please run: gcloud auth login",
    ];
    let notice = Some(NOTICES);

    assert_eq!(
        told("gcloud", &lines),
        [None, notice, notice, notice, notice, notice, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    let lines = [
        "src/app.ts 45ms (unchanged)",
        "For more information:",
        "Tip: Want to turn off tips and other hints?",
        "LOAD   = Reflects whether the unit definition was properly loaded.",
        "please run:",
    ];

    assert_eq!(told("eslint", &lines), [None; 5]);
    assert_eq!(
        told("prettier", &lines),
        [Some(UNCHANGED_FILES), None, None, None, None]
    );
    assert_eq!(
        told("gcloud", &lines),
        [None, None, None, None, Some(NOTICES)]
    );
}
