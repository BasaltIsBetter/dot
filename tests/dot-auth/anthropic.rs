use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";
const ANY_LOCAL_PORT: &str = "127.0.0.1:0";
const LONGEST_SILENCE: Duration = Duration::from_secs(3);

struct LocalServer {
    url: String,
    request: thread::JoinHandle<Vec<String>>,
}

fn answer_once(status_line: &'static str) -> LocalServer {
    let listener = TcpListener::bind(ANY_LOCAL_PORT).unwrap();
    let url = format!("http://{}/v1/models", listener.local_addr().unwrap());
    let request = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request: Vec<String> = BufReader::new(stream.try_clone().unwrap())
            .lines()
            .map(Result::unwrap)
            .take_while(|line| !line.is_empty())
            .collect();
        write!(stream, "{status_line}\r\ncontent-length: 2\r\n\r\n{{}}").unwrap();
        request
    });
    LocalServer { url, request }
}

#[test]
fn accepts_a_success() {
    assert!(matches!(verdict(200), Verdict::Accepted));
}

#[test]
fn rejects_a_key_anthropic_refuses() {
    assert!(matches!(verdict(401), Verdict::Rejected));
    assert!(matches!(verdict(403), Verdict::Rejected));
}

#[test]
fn cannot_tell_when_anthropic_fails_on_its_side() {
    for status in [301, 400, 429, 500, 529] {
        let Verdict::Unreachable(cause) = verdict(status) else {
            panic!("{status} should be unreachable");
        };
        assert_eq!(
            cause.to_string(),
            format!("Anthropic answered with HTTP status {status}")
        );
    }
}

#[test]
fn sends_the_key_and_the_api_version_in_headers_only() {
    let server = answer_once("HTTP/1.1 401 Unauthorized");

    let verdict =
        HttpKeyVerifier::at(server.url).verify(&ApiKey::parse(KEY).unwrap(), &Cancellation::new());

    let request = server.request.join().unwrap();
    assert!(matches!(verdict, Verdict::Rejected));
    assert_eq!(request[0], "GET /v1/models HTTP/1.1");
    assert!(request.contains(&format!("{KEY_HEADER}: {KEY}")));
    assert!(request.contains(&format!("{VERSION_HEADER}: {API_VERSION}")));
}

#[test]
fn never_follows_a_redirect_with_the_key() {
    let server = answer_once("HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:1/");

    let verdict =
        HttpKeyVerifier::at(server.url).verify(&ApiKey::parse(KEY).unwrap(), &Cancellation::new());

    server.request.join().unwrap();
    assert!(matches!(verdict, Verdict::Unreachable(_)));
}

#[test]
#[ignore = "calls api.anthropic.com over the network"]
fn anthropic_rejects_a_made_up_key() {
    let made_up = ApiKey::parse("sk-ant-api03-not-a-real-key-0000000000").unwrap();

    let verdict = HttpKeyVerifier::new().verify(&made_up, &Cancellation::new());

    assert!(matches!(verdict, Verdict::Rejected), "{verdict:?}");
}

#[test]
fn cannot_tell_when_nothing_answers() {
    let verifier = HttpKeyVerifier::at("http://127.0.0.1:1/v1/models");

    assert!(matches!(
        verifier.verify(&ApiKey::parse(KEY).unwrap(), &Cancellation::new()),
        Verdict::Unreachable(_)
    ));
}

#[test]
fn stops_waiting_for_anthropic_as_soon_as_it_is_cancelled() {
    let listener = TcpListener::bind(ANY_LOCAL_PORT).unwrap();
    let url = format!("http://{}/v1/models", listener.local_addr().unwrap());
    let (release, released) = mpsc::channel::<()>();
    let silent = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        released.recv_timeout(LONGEST_SILENCE).unwrap_err();
        drop(stream);
    });
    let cancellation = Cancellation::new();
    let cancelling = cancellation.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        cancelling.cancel();
    });
    let started = std::time::Instant::now();

    let verdict = HttpKeyVerifier::at(url).verify(&ApiKey::parse(KEY).unwrap(), &cancellation);
    let waited = started.elapsed();
    drop(release);

    assert!(matches!(verdict, Verdict::Unreachable(_)));
    assert!(waited < Duration::from_secs(2), "{waited:?}");
    canceller.join().unwrap();
    silent.join().unwrap();
}
