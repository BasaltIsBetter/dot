use super::*;

#[test]
fn recognises_a_diff_by_its_headers() {
    assert!(is_diff("diff --git a/x b/x\nindex 1..2\n"));
    assert!(is_diff("--- a\n+++ b\n@@ -1,3 +1,4 @@\n context\n"));
}

#[test]
fn does_not_take_other_text_for_a_diff() {
    assert!(!is_diff("@@ not a hunk\nplain text\n"));
    assert!(!is_diff("@@ -1,3 never closed\n"));
    assert!(!is_diff("text with @@ in it\n"));
    assert!(!is_diff("see diff --git in the manual\n"));
    assert!(!is_diff(""));
}
