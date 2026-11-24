use super::*;

#[test]
fn keeps_each_event_smaller_than_the_change_of_a_file() {
    assert!(
        size_of::<ReplyEvent>() < size_of::<FileChange>(),
        "{} {}",
        size_of::<ReplyEvent>(),
        size_of::<FileChange>()
    );
}
