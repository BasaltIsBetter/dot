use proptest::prelude::*;

use super::*;

fn unfolded(grouped: &str) -> Vec<String> {
    let mut common = "";
    let mut paths = Vec::new();
    for line in grouped.lines() {
        if let Some(folder) = line
            .strip_prefix("[paths under ")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            common = folder;
        } else if let Some((folder, names)) =
            line.strip_suffix('}').and_then(|line| line.split_once('{'))
        {
            paths.extend(
                names
                    .split(',')
                    .map(|name| format!("{common}{folder}{name}")),
            );
        } else {
            paths.push(format!("{common}{line}"));
        }
    }
    paths
}

#[test]
fn folds_the_files_of_a_folder_with_braces() {
    let found = "./src/a.rs\n./src/b.rs\n./src/c.rs\n./src/sub/d.rs\n./lib/e.rs\n./lib/f.rs\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/{a.rs,b.rs,c.rs}\nsrc/sub/d.rs\nlib/{e.rs,f.rs}\n")
    );
}

#[test]
fn keeps_the_order_of_the_list() {
    let found = "src/a.rs\nlib/b.rs\nsrc/c.rs\nlib/d.rs\nsrc/e.rs\nsrc/f.rs\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/a.rs\nlib/b.rs\nsrc/c.rs\nlib/d.rs\nsrc/{e.rs,f.rs}\n")
    );
    assert_eq!(
        group("src/a.rs\nlib/b.rs\nsrc/c.rs\nlib/d.rs\nsrc/e.rs\nlib/f.rs\n"),
        None
    );
}

#[test]
fn names_the_folder_absolute_paths_share() {
    let found = "/Users/me/project/src/a.rs\n/Users/me/project/src/b.rs\n/Users/me/project/c.rs\n/Users/me/project/lib/d.rs\n/Users/me/project/lib/e.rs\n/Users/me/project/lib/f.rs\n";

    assert_eq!(
        group(found).as_deref(),
        Some("[paths under /Users/me/project/]\nsrc/{a.rs,b.rs}\nc.rs\nlib/{d.rs,e.rs,f.rs}\n")
    );
}

#[test]
fn does_not_fold_names_that_braces_would_split() {
    let found = "src/a,b.rs\nsrc/c.rs\nsrc/d.rs\nsrc/e.rs\nsrc/f.rs\nsrc/g.rs\n";

    assert_eq!(group(found), None);
    assert_eq!(group("x/a{1}\nx/b\nx/c\nx/d\nx/e\nx/f\n"), None);
}

#[test]
fn lists_folders_themselves_on_their_own_lines() {
    let found = "./src/\n./src/a.rs\n./src/b.rs\n./src/c.rs\n./lib/\n./lib/d.rs\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/\nsrc/a.rs\nsrc/b.rs\nsrc/c.rs\nlib/\nlib/d.rs\n")
    );
}

#[test]
fn leaves_alone_what_is_not_mostly_a_list_of_paths() {
    assert_eq!(
        group("src/a.rs\nsrc/b.rs\nsrc/c.rs\nsrc/d.rs\nsrc/e.rs\n"),
        None
    );
    assert_eq!(
        group("src/a.rs\nsrc/b.rs\nsrc/c.rs\nsrc/d.rs\nsrc/e.rs\nsrc/f.rs\nsome text\nmore text\n"),
        None
    );
    assert_eq!(group("a\nb\nc\nd\ne\nf\ng\n"), None);
}

#[test]
fn does_not_take_matches_options_or_sentences_for_paths() {
    assert_eq!(path_of("src/a.rs:12:text"), None);
    assert_eq!(path_of("--path/like"), None);
    assert_eq!(path_of("see src/a.rs"), None);
    assert_eq!(path_of("[paths under /x/]"), None);
    assert_eq!(path_of("Cargo.toml"), None);
    assert_eq!(path_of("./src/a.rs"), Some("src/a.rs"));
}

#[test]
fn keeps_other_lines_where_they_are() {
    let found = "src/a.rs\nsrc/b.rs\n\nsrc/c.rs\nsrc/d.rs\nlib/e.rs\nlib/f.rs\nlib/g.rs\nlib/h.rs\nlib/i.rs\nlib/j.rs\n(10 files)";

    assert_eq!(
        group(found).as_deref(),
        Some("src/{a.rs,b.rs}\n\nsrc/{c.rs,d.rs}\nlib/{e.rs,f.rs,g.rs,h.rs,i.rs,j.rs}\n(10 files)")
    );
}

proptest! {
    #[test]
    fn every_path_is_told_in_the_same_order(
        entries in prop::collection::vec((0_usize..5, "[a-z_]{1,10}(\\.rs)?"), 6..60),
        absolute in any::<bool>(),
    ) {
        let root = if absolute { "/Users/me/a-long-project-folder/" } else { "" };
        let folders = ["src/", "src/deep/", "lib/", "tests/a/b/", "x/"];
        let paths: Vec<String> = entries.iter().map(|(folder, name)| format!("{root}{}{name}", folders[*folder])).collect();
        let found = paths.join("\n");

        if let Some(grouped) = group(&found) {
            prop_assert!(grouped.len() < found.len());
            prop_assert_eq!(unfolded(&grouped), paths);
        }
    }

    #[test]
    fn any_text_is_grouped_without_panicking(text in any::<String>()) {
        if let Some(grouped) = group(&text) {
            prop_assert!(grouped.len() < text.len());
        }
    }
}
