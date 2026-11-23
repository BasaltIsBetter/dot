use super::*;

pub(crate) fn attachment(label: &str) -> Attachment {
    Attachment {
        kind: "image".to_owned(),
        name: label.to_owned(),
        size: "2 KB".to_owned(),
        image: Image::from_bytes(b"\x89PNG\r\n\x1a\n".to_vec()).unwrap(),
    }
}

#[test]
fn writes_the_kind_the_name_and_the_size_two_columns_apart() {
    let label = attachment("a.png").label(TextWidth::new(40));

    assert_eq!(label, "image  a.png  2 KB");
}

#[test]
fn shortens_the_name_of_an_image_to_fit_a_narrow_column() {
    let label = attachment(&"long-name-".repeat(10)).label(TextWidth::new(28));

    assert_eq!(columns(&label), 28);
    assert!(label.contains('…'));
}
