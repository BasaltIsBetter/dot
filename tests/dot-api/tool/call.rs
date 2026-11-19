use super::*;

#[test]
fn a_search_gives_back_the_matching_files_unless_told_otherwise() {
    assert_eq!(SearchShow::default(), SearchShow::Files);
}
