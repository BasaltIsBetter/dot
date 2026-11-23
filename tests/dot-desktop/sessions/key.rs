use std::sync::mpsc;

use super::*;
use crate::event::Delivery;
use crate::reply::tests::news_of;

#[test]
fn tags_the_news_of_each_session_with_its_key() {
    let (sender, receiver) = mpsc::channel::<Delivery>();
    let tagged = Tagged::new(sender, SessionKey(7));

    tagged.deliver(news_of("hello"));

    assert!(matches!(
        receiver.try_recv(),
        Ok(Delivery::Reply(SessionKey(7), _))
    ));
}
