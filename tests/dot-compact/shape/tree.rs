use proptest::prelude::*;

use super::*;

const TREE: &str = ".\n├── Cargo.toml\n├── crates\n│   ├── dot\n│   │   └── src\n│   │       └── main.rs\n│   └── dot-api\n└── README.md\n\n3 directories, 3 files\n";

#[test]
fn writes_a_level_as_one_space() {
    assert_eq!(
        flatten(TREE).as_deref(),
        Some(
            "[tree as: one space per level]\n.\n Cargo.toml\n crates\n  dot\n   src\n    main.rs\n  dot-api\n README.md\n\n3 directories, 3 files\n"
        )
    );
}

#[test]
fn keeps_the_sections_cargo_hangs_in_its_tree() {
    let tree = "dot v0.1.0 (/work/dot)\n├── serde v1.0.228\n│   └── serde_derive v1.0.228 (proc-macro)\n│       [build-dependencies]\n│       └── autocfg v1.5.0\n└── ureq v3.1.2\n[dev-dependencies]\n└── proptest v1.8.0";

    assert_eq!(
        flatten(tree).as_deref(),
        Some(
            "[tree as: one space per level]\ndot v0.1.0 (/work/dot)\n serde v1.0.228\n  serde_derive v1.0.228 (proc-macro)\n  [build-dependencies]\n   autocfg v1.5.0\n ureq v3.1.2\n[dev-dependencies]\n proptest v1.8.0"
        )
    );
}

#[test]
fn reads_the_unbreakable_spaces_tree_draws() {
    let tree = "src\n├── a\n│\u{a0}\u{a0} ├── b\n│\u{a0}\u{a0} └── c\n└── d\n";

    assert_eq!(
        flatten(tree).as_deref(),
        Some("[tree as: one space per level]\nsrc\n a\n  b\n  c\n d\n")
    );
}

#[test]
fn leaves_alone_what_is_written_around_the_tree() {
    let tree = "    an indented heading\n├── a\n├── b\n├── c\n└── d\n    an indented note\n";

    assert_eq!(
        flatten(tree).as_deref(),
        Some(
            "[tree as: one space per level]\n    an indented heading\n a\n b\n c\n d\n    an indented note\n"
        )
    );
}

#[test]
fn keeps_a_tree_drawn_another_way_as_it_is() {
    let two_columns = "app@1.0.0 /work/app\n├── left-pad@1.3.0\n├─┬ chalk@5.3.0\n│ └── ansi-styles@6.2.1\n├── lodash@4.17.21\n├── semver@7.6.0\n└── tslib@2.6.2\n";
    let short_branches = ".\n├── a\n├── b\n│   ├─ c\n├── d\n└── e\n";

    assert_eq!(flatten(two_columns), None);
    assert_eq!(flatten(short_branches), None);
}

#[test]
fn a_drawing_outside_the_tree_does_not_stop_it() {
    let tree = "│ a note that draws a bar\n├── a\n├── b\n├── c\n└── d\n└ another one\n";

    assert_eq!(
        flatten(tree).as_deref(),
        Some(
            "[tree as: one space per level]\n│ a note that draws a bar\n a\n b\n c\n d\n└ another one\n"
        )
    );
}

#[test]
fn needs_four_branches_to_rewrite_a_tree() {
    assert_eq!(flatten(".\n├── a\n├── b\n└── c\n"), None);
    assert!(flatten(".\n├── a\n├── b\n├── c\n└── d\n").is_some());
    assert_eq!(flatten("no tree\n    at all\n"), None);
    assert_eq!(flatten(""), None);
}

#[test]
fn a_level_is_a_trunk_or_a_gap_four_columns_wide() {
    assert_eq!(below_the_trunks("│   │   name"), (2, "name"));
    assert_eq!(below_the_trunks("    │   ├── name"), (2, "├── name"));
    assert_eq!(below_the_trunks("│  name"), (0, "│  name"));
    assert_eq!(below_the_trunks("│   "), (1, ""));
    assert_eq!(below_the_trunks("├── name"), (0, "├── name"));
    assert_eq!(below_the_trunks(""), (0, ""));
}

fn drawn(nodes: &[(usize, String)]) -> String {
    let is_last = |at: usize| {
        let depth = nodes[at].0;
        nodes[at + 1..]
            .iter()
            .map(|(next, _)| *next)
            .take_while(|next| *next >= depth)
            .all(|next| next != depth)
    };
    let mut text = String::from(".\n");
    for (at, (depth, name)) in nodes.iter().enumerate() {
        for level in 1..*depth {
            let ancestor = (0..at)
                .rev()
                .find(|before| nodes[*before].0 == level)
                .unwrap();
            text.push_str(if is_last(ancestor) { "    " } else { "│   " });
        }
        text.push_str(if is_last(at) {
            "└── "
        } else {
            "├── "
        });
        text.push_str(name);
        text.push('\n');
    }
    text
}

fn nodes() -> impl Strategy<Value = Vec<(usize, String)>> {
    prop::collection::vec((0_usize..4, "[a-z]{1,8}"), 4..40).prop_map(|raw| {
        let mut previous = 0;
        raw.into_iter()
            .map(|(wanted, name)| {
                previous = (previous + 1).min(wanted + 1);
                (previous, name)
            })
            .collect()
    })
}

proptest! {
    #[test]
    fn tells_the_same_tree(nodes in nodes()) {
        let flat = flatten(&drawn(&nodes)).unwrap();

        let told: Vec<String> = flat.lines().skip(2).map(str::to_owned).collect();
        let expected: Vec<String> = nodes
            .iter()
            .map(|(depth, name)| format!("{}{name}", " ".repeat(*depth)))
            .collect();
        prop_assert_eq!(told, expected);
        prop_assert!(flat.len() < drawn(&nodes).len());
    }

    #[test]
    fn any_text_is_read_without_panicking(text in "[│├└─ \u{a0}a-z\n]{0,200}") {
        if let Some(flat) = flatten(&text) {
            prop_assert!(flat.starts_with(NOTE));
        }
    }
}
