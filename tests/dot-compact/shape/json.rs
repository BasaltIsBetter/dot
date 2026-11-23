use proptest::prelude::*;

use super::*;

#[test]
fn removes_the_spaces_that_only_indent() {
    let pretty = "{\n  \"name\": \"dot desktop\",\n  \"tags\": [\n    1,\n    2\n  ]\n}\n";

    assert_eq!(
        minify(pretty).as_deref(),
        Some(r#"{"name":"dot desktop","tags":[1,2]}"#)
    );
}

#[test]
fn keeps_what_is_inside_strings() {
    let pretty = "[\n\t\"a \\\" b\",\n\t\"c\\\\\",\n\t\" { } [ ] : , \"\n]";

    assert_eq!(
        minify(pretty).as_deref(),
        Some(r#"["a \" b","c\\"," { } [ ] : , "]"#)
    );
}

#[test]
fn leaves_alone_what_is_already_short_or_is_not_json() {
    assert_eq!(minify(r#"{"a": 1, "b": [1, 2]}"#), None);
    assert_eq!(minify("{\n  not json\n}"), None);
    assert_eq!(minify("{\n  \"a\": 1\n}\ntrailing text"), None);
    assert_eq!(minify("plain\n text"), None);
    assert_eq!(minify(""), None);
}

#[test]
fn removes_the_indentation_of_a_document_that_follows_other_lines() {
    let output = "[+3 lines: transfer progress]\n{\n  \"name\": \"dot\",\n  \"items\": [\n    1,\n    2\n  ]\n}\n";

    assert_eq!(
        minify_after_lines(output).as_deref(),
        Some("[+3 lines: transfer progress]\n{\"name\":\"dot\",\"items\":[1,2]}")
    );
    assert_eq!(
        minify_after_lines("warning: slow\r\n[\r\n  1,\r\n  2\r\n]").as_deref(),
        Some("warning: slow\r\n[1,2]")
    );
}

#[test]
fn only_reads_a_document_that_opens_on_its_own_line_and_runs_to_the_end() {
    for output in [
        "{\n  \"a\": 1\n}\n",
        "note\n{\n  \"a\": 1\n}\ntrailing\n",
        "note\n{\"a\": {\n  \"b\": 1\n}}\n",
        "note\n{\n  not json\n}\n",
        "note\nplain\n",
        "",
    ] {
        assert_eq!(minify_after_lines(output), None, "{output}");
    }
}

fn after_brackets(count: usize) -> String {
    format!("note\n{}{{\n  \"a\": 1\n}}", "[\n".repeat(count))
}

#[test]
fn reads_past_brackets_that_open_nothing_but_not_for_ever() {
    assert_eq!(
        minify_after_lines(&after_brackets(1)).as_deref(),
        Some("note\n[\n{\"a\":1}")
    );
    assert!(minify_after_lines(&after_brackets(15)).is_some());
    assert_eq!(minify_after_lines(&after_brackets(16)), None);
}

#[test]
fn knows_a_whole_document_whether_it_is_indented_or_not() {
    assert!(is_document(r#"{"a":1,"b":[1,2]}"#));
    assert!(is_document("  [1, 2]\n"));
    assert!(is_document("{\n  \"a\": 1\n}"));
    assert!(!is_document(r#"{"a":1} trailing"#));
    assert!(!is_document("{\"a\":1}\n{\"b\":2}"));
    assert!(!is_document("{not json"));
    assert!(!is_document(r#""a string""#));
    assert!(!is_document("12"));
    assert!(!is_document(""));
}

fn value() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::from),
        any::<i64>().prop_map(serde_json::Value::from),
        any::<String>().prop_map(serde_json::Value::from),
    ];
    leaf.prop_recursive(3, 24, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(serde_json::Value::from),
            prop::collection::btree_map(any::<String>(), inner, 0..6)
                .prop_map(|map| serde_json::Value::Object(map.into_iter().collect())),
        ]
    })
}

proptest! {
    #[test]
    fn a_list_or_an_object_is_a_document_written_whole_or_indented(document in value()) {
        let is_a_container = document.is_array() || document.is_object();

        prop_assert_eq!(is_document(&serde_json::to_string(&document).unwrap()), is_a_container);
        prop_assert_eq!(
            is_document(&serde_json::to_string_pretty(&document).unwrap()),
            is_a_container
        );
    }

    #[test]
    fn says_the_same_as_the_indented_document(document in value()) {
        let pretty = serde_json::to_string_pretty(&document).unwrap();

        if let Some(minified) = minify(&pretty) {
            prop_assert!(minified.len() <= pretty.len());
            prop_assert_eq!(serde_json::from_str::<serde_json::Value>(&minified).unwrap(), document);
        }
    }

    #[test]
    fn any_text_is_read_without_panicking(text in any::<String>()) {
        if is_document(&text) {
            prop_assert!(serde_json::from_str::<serde_json::Value>(text.trim()).is_ok());
        }
        if let Some(minified) = minify(&text) {
            prop_assert!(minified.len() <= text.len());
        }
        if let Some(minified) = minify_after_lines(&text) {
            prop_assert!(minified.len() <= text.len());
        }
    }

    #[test]
    fn says_the_same_as_the_document_under_other_lines(
        document in value(),
        before in "[a-z :]{1,20}\n",
    ) {
        let pretty = serde_json::to_string_pretty(&document).unwrap();

        if let Some(minified) = minify_after_lines(&format!("{before}{pretty}")) {
            let told = minified.strip_prefix(&before).unwrap();
            prop_assert_eq!(serde_json::from_str::<serde_json::Value>(told).unwrap(), document);
        }
    }
}
