use dot_api::{Detail, Image};
use serde_json::json;

use super::*;

#[test]
fn answers_each_tool_by_its_id_marking_errors_only() {
    let results = vec![
        (
            "toolu_1".to_owned(),
            ToolOutput::done("3 files", Detail::Files(3)),
        ),
        ("toolu_2".to_owned(), ToolOutput::failed("no such file")),
    ];

    let message = serde_json::to_value(ToolResults::of(&results)).unwrap();

    assert_eq!(
        message,
        json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": "3 files"},
            {"type": "tool_result", "tool_use_id": "toolu_2", "content": "no such file",
             "is_error": true},
        ]})
    );
    assert!(!ToolResults::of(&results).is_empty());
    assert!(ToolResults::of(&[]).is_empty());
}

#[test]
fn hands_an_image_over_as_an_image_block() {
    let png = Image::from_bytes(b"\x89PNG\r\n\x1a\n".to_vec()).unwrap();
    let results = vec![(
        "toolu_1".to_owned(),
        ToolOutput {
            content: ToolContent::Image(png),
            is_error: false,
            end: dot_api::ToolEnd::Done(Detail::Nothing),
            change: None,
        },
    )];

    let message = serde_json::to_value(ToolResults::of(&results)).unwrap();

    assert_eq!(
        message["content"][0]["content"],
        json!([{"type": "image", "source": {"type": "base64", "media_type": "image/png",
            "data": "iVBORw0KGgo="}}])
    );
}
