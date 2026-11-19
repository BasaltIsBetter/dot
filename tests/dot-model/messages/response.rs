use std::io::Cursor;

use proptest::prelude::*;

use super::*;

pub(crate) const REPLY: &str = "event: message_start
data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-opus-5-5\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}

event: ping
data: {\"type\":\"ping\"}

event: content_block_start
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}

event: content_block_delta
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Bonjour\"}}

event: content_block_stop
data: {\"type\":\"content_block_stop\",\"index\":0}

event: message_delta
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}

event: message_stop
data: {\"type\":\"message_stop\"}

";

fn read(stream: &str, cancellation: &Cancellation) -> (Result<(), ReplyError>, Vec<ReplyEvent>) {
    let mut turn = Turn::replayable(None);
    let mut heard = Vec::new();
    let result = read_stream(
        Cursor::new(stream.as_bytes().to_vec()),
        &mut turn,
        &mut |event| heard.push(event),
        cancellation,
    );
    (result, heard)
}

#[test]
fn reads_a_whole_reply() {
    let (result, heard) = read(REPLY, &Cancellation::new());

    assert!(result.is_ok());
    assert_eq!(heard, [ReplyEvent::Text("Bonjour".into())]);
}

#[test]
fn reports_a_stream_cut_before_the_end() {
    let cut = &REPLY[..REPLY.find("event: message_stop").unwrap()];

    let (result, heard) = read(cut, &Cancellation::new());

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Unreachable);
    assert_eq!(heard.len(), 1);
}

#[test]
fn reports_a_stopped_reply_as_stopped() {
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let (result, _) = read(&REPLY[..200], &cancellation);

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Stopped);
}

#[test]
fn reports_an_error_sent_in_the_stream() {
    let failing = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";

    let (result, _) = read(failing, &Cancellation::new());

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Overloaded);
}

#[test]
fn reports_a_stream_it_cannot_read() {
    let (result, _) = read("data: {not json\n\n", &Cancellation::new());

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Unreachable);
}

#[test]
fn reads_a_stream_line_up_to_four_mebibytes() {
    let padding = " ".repeat(4 * 1_024 * 1_024 - 200);
    let padded = REPLY.replacen(
        "data: {\"type\":\"content_block_delta\"",
        &format!("data: {padding}{{\"type\":\"content_block_delta\""),
        1,
    );

    let (result, heard) = read(&padded, &Cancellation::new());

    assert!(result.is_ok());
    assert_eq!(heard, [ReplyEvent::Text("Bonjour".into())]);
}

#[test]
fn reads_the_detail_of_an_error_body_up_to_sixty_four_kibibytes() {
    let padding = " ".repeat(64 * 1_024 - 200);
    let body = format!(
        "{{\"type\":\"error\",{padding}\"error\":{{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}}}"
    );
    let response = Response::builder()
        .status(529)
        .body(Body::builder().data(body))
        .unwrap();

    let error = status_error(response);

    assert_eq!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .map(ToString::to_string),
        Some("Anthropic answered with overloaded_error: Overloaded".to_owned())
    );
}

#[test]
fn tells_each_status_apart() {
    let statuses = [
        (400, ReplyFailure::Rejected),
        (401, ReplyFailure::Unauthorized),
        (403, ReplyFailure::Unauthorized),
        (404, ReplyFailure::Rejected),
        (413, ReplyFailure::Rejected),
        (429, ReplyFailure::RateLimited),
        (500, ReplyFailure::Unreachable),
        (503, ReplyFailure::Overloaded),
        (529, ReplyFailure::Overloaded),
        (302, ReplyFailure::Unreachable),
    ];

    for (status, failure) in statuses {
        assert_eq!(failure_of_status(status), failure, "{status}");
    }
}

#[test]
fn retries_what_may_pass_a_moment_later() {
    for status in [408, 409, 429, 500, 503, 529] {
        assert_eq!(
            retry_delay(Some(status), None, 0),
            Some(FIRST_RETRY_DELAY),
            "{status}"
        );
    }
    assert_eq!(retry_delay(None, None, 0), Some(FIRST_RETRY_DELAY));
}

#[test]
fn never_retries_a_refusal_of_the_request() {
    for status in [400, 401, 403, 404, 413] {
        assert_eq!(retry_delay(Some(status), None, 0), None, "{status}");
    }
}

#[test]
fn waits_longer_each_time_then_gives_up() {
    assert_eq!(retry_delay(Some(529), None, 1), Some(FIRST_RETRY_DELAY * 2));
    assert_eq!(retry_delay(Some(529), None, RETRIES), None);
}

#[test]
fn waits_as_long_as_anthropic_asks_within_reason() {
    assert_eq!(
        retry_delay(Some(429), Some("3"), 0),
        Some(Duration::from_secs(3))
    );
    assert_eq!(
        retry_delay(Some(429), Some("20"), 0),
        Some(LONGEST_RETRY_DELAY)
    );
    assert_eq!(retry_delay(Some(429), Some("21"), 0), None);
    assert_eq!(
        retry_delay(Some(429), Some("soon"), 0),
        Some(FIRST_RETRY_DELAY)
    );
}

#[test]
fn pauses_until_the_delay_or_the_cancellation() {
    let started = Instant::now();
    assert!(pause(Duration::from_millis(60), &Cancellation::new()));
    assert!(started.elapsed() >= Duration::from_millis(60));

    let cancellation = Cancellation::new();
    cancellation.cancel();
    let started = Instant::now();
    assert!(!pause(Duration::from_secs(5), &cancellation));
    assert!(started.elapsed() < PAUSE_STEP);
}

proptest! {
    #[test]
    fn reads_any_stream_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..600)) {
        let mut turn = Turn::replayable(None);
        let result = read_stream(
            Cursor::new(bytes),
            &mut turn,
            &mut |_| {},
            &Cancellation::new(),
        );
        prop_assert!(result.is_err() || turn.is_finished());
    }

    #[test]
    fn reads_the_same_reply_however_it_is_split(split in 1_usize..64) {
        let mut turn = Turn::replayable(None);
        let mut heard = Vec::new();
        let chunked = std::io::BufReader::with_capacity(split, Cursor::new(REPLY.as_bytes().to_vec()));

        read_stream(chunked, &mut turn, &mut |event| heard.push(event), &Cancellation::new()).unwrap();

        prop_assert_eq!(heard, vec![ReplyEvent::Text("Bonjour".into())]);
    }
}
