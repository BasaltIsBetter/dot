use serde_json::Value;

use super::*;

#[test]
fn declares_each_tool_once_in_valid_json() {
    let tools: Value = serde_json::from_str(TOOLS).unwrap();

    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "bash",
            "str_replace_based_edit_tool",
            "find_files",
            "search_text",
            "web_search",
            "web_fetch"
        ]
    );
    assert_eq!(tools[0]["type"], "bash_20250124");
    assert_eq!(tools[1]["type"], "text_editor_20250728");
    assert_eq!(tools[4]["type"], "web_search_20260209");
    assert_eq!(tools[5]["type"], "web_fetch_20260209");
    for custom in [&tools[2], &tools[3]] {
        assert_eq!(
            custom["input_schema"]["required"],
            serde_json::json!(["pattern"])
        );
        assert_eq!(custom["input_schema"]["additionalProperties"], false);
    }
}

#[test]
fn tells_claude_where_the_project_is() {
    let prompt = system_prompt(Some(Path::new("/Users/me/Dot")));

    assert!(prompt.starts_with(INTRODUCTION));
    assert!(
        prompt.contains("The user's project is the folder /Users/me/Dot. Commands start there.")
    );
    assert!(prompt.ends_with(TOOL_GUIDE));
    assert!(!prompt.contains(NO_FOLDER));
}

#[test]
fn asks_for_absolute_paths_without_a_project() {
    let prompt = system_prompt(None);

    assert!(prompt.contains(NO_FOLDER));
    assert!(!prompt.contains("The user's project"));
}

#[test]
fn tells_claude_code_how_dot_shows_its_replies() {
    assert!(CLAUDE_CODE_NOTE.contains("does not render Markdown"));
    assert!(CLAUDE_CODE_NOTE.contains("do not try to get around the refusal"));
}
