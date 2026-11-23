use super::*;

#[test]
fn lists_the_lines_that_open_a_block_at_the_shallowest_level() {
    let cut = "    }\n\n    fn first(&self) {\n        let x = 1;\n        if x > 0 {\n            go();\n        }\n    }\n\n    #[test]\n    fn second() {\n        stop();\n    }\n}\n";

    assert_eq!(
        of(cut, 100, 1_000),
        ["102:    fn first(&self) {", "110:    fn second() {"]
    );
}

#[test]
fn reads_a_brace_on_its_own_line_as_the_block_of_the_line_above() {
    let cut = "void first()\n{\n    body();\n}\nint x;\nvoid second()\n{\n}\n";

    assert_eq!(of(cut, 1, 1_000), ["1:void first()", "6:void second()"]);
}

#[test]
fn finds_nothing_to_list_in_flat_text_or_in_brackets_alone() {
    assert_eq!(of("one\ntwo\nthree\n", 1, 1_000), [""; 0]);
    assert_eq!(of("{\n    {\n        deep\n", 1, 1_000), [""; 0]);
    assert_eq!(of("", 1, 1_000), [""; 0]);
    assert_eq!(of("last line opens nothing {\n", 1, 1_000), [""; 0]);
}

#[test]
fn lists_forty_lines_at_most_and_no_more_bytes_than_it_may() {
    let blocks = (1..=50)
        .map(|block| format!("block {block:02}:\n  inside\n"))
        .collect::<Vec<String>>()
        .concat();

    let listed = of(&blocks, 1, 10_000);

    assert_eq!(listed.len(), 40);
    assert_eq!(listed[39], "79:block 40:");
    assert_eq!(of(&blocks, 1, 22), ["1:block 01:", "3:block 02:"]);
    assert_eq!(of(&blocks, 1, 21), ["1:block 01:"]);
    assert_eq!(of(&blocks, 1, 10), [""; 0]);
}

#[test]
fn cuts_a_long_opening_line_and_drops_what_trails_it() {
    let cut = format!("fn long({}) {{   \n    body();\n", "a".repeat(200));

    assert_eq!(
        of(&cut, 7, 1_000),
        [format!("7:fn long({}[+91 chars]", "a".repeat(112))]
    );
    assert_eq!(
        of("fn short() {  \n    body();\n", 7, 1_000),
        ["7:fn short() {"]
    );
}

#[test]
fn lists_the_headings_of_a_document_rather_than_its_blocks() {
    let cut = "text\n\n## Rules\n\n- one:\n  - nested\n\n### 2026-11-19 — Title\n**Why**: because\n\n# Top\n\n####### seven\n\n##no space\n";

    assert_eq!(
        of(cut, 10, 1_000),
        ["12:## Rules", "17:### 2026-11-19 — Title"]
    );
}

#[test]
fn a_heading_follows_an_empty_line_or_starts_the_cut() {
    let starts = "## First\ntext\n## glued to the text\n\n## Second\n";
    let comments = "fn a() {\n    b();\n}\n## not a heading here\nfn c() {\n    d();\n}\n";

    assert_eq!(of(starts, 1, 1_000), ["1:## First", "5:## Second"]);
    assert_eq!(of(comments, 1, 1_000), ["1:fn a() {", "5:fn c() {"]);
}
