use super::*;
use crate::fleet::tests::Fake;

const ANSWERS: &str = r#"cat > "$HERE/prompt"
echo '{"type":"assistant","message":{"content":[{"type":"text","text":"I look."}]}}'
echo '{"type":"result","is_error":false,"result":"Done."}'"#;
const HANGS: &str = r#"case "$*" in *dot-end-call*) : > "$HERE/ended" ;; *) sleep 30 ;; esac"#;

fn called(fake: &Fake, cancellation: &Cancellation) -> Result<Answer, CallError> {
    let call = Call {
        prompt: "Review this.",
        goes_on: false,
    };
    fake.fleet.call(
        (&fake.name, &fake.spec),
        &fake.project,
        &call,
        cancellation,
        &mut |_| {},
    )
}

#[test]
fn hands_the_prompt_and_returns_the_answer() {
    let fake = Fake::new("call-answer", ANSWERS);

    let answer = called(&fake, &Cancellation::new()).unwrap();

    assert_eq!(answer.text, "Done.");
    assert_eq!(fake.scratch.read("prompt"), "Review this.");
}

#[test]
fn tells_why_an_agent_failed() {
    let fake = Fake::new("call-failed", "echo 'claude: not found' >&2; exit 127");

    let failed = called(&fake, &Cancellation::new()).unwrap_err();

    assert!(failed.to_string().contains("claude: not found"));
}

#[test]
fn an_answer_from_a_program_that_fails_is_no_answer() {
    let mut fake = Fake::new("call-program", "echo half; exit 3");
    fake.spec.command = vec!["count".to_owned()];

    assert!(matches!(
        called(&fake, &Cancellation::new()),
        Err(CallError::Failed(_))
    ));
}

#[test]
fn stopping_a_call_ends_it_inside_the_container_too() {
    let fake = Fake::new("call-stopped", HANGS);
    let cancellation = Cancellation::new();
    let started = Instant::now();

    let stopped = thread::scope(|scope| {
        let handle = scope.spawn(|| called(&fake, &cancellation));
        while fake.runs_of("exec") == 0 && started.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(5));
        }
        cancellation.cancel();
        handle.join().unwrap()
    });

    assert!(matches!(stopped, Err(CallError::Stopped)));
    assert!(fake.scratch.path().join("ended").exists());
    assert!(started.elapsed() < Duration::from_secs(15));
    let log = fake.scratch.read("log");
    let mark = log.split_once("DOT_CALL=").unwrap().1;
    let mark = &mark[..32];
    assert!(log.contains(&format!("dot-end-call {mark} TERM")));
    assert!(log.contains(&format!("dot-end-call {mark} KILL")));
}

#[test]
fn a_program_that_goes_on_after_its_answer_can_still_be_stopped() {
    let silent = r#"case "$*" in *dot-end-call*) : > "$HERE/ended" ;; *) exec >&-; : > "$HERE/silent"; sleep 30 ;; esac"#;
    let fake = Fake::new("call-silent", silent);
    let cancellation = Cancellation::new();
    let started = Instant::now();

    let stopped = thread::scope(|scope| {
        let handle = scope.spawn(|| called(&fake, &cancellation));
        let silent = fake.scratch.path().join("silent");
        while !silent.exists() && started.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(LOOK * 2);
        cancellation.cancel();
        handle.join().unwrap()
    });

    assert!(matches!(stopped, Err(CallError::Stopped)));
    assert!(fake.scratch.path().join("ended").exists());
    assert!(started.elapsed() < Duration::from_secs(15));
}

#[test]
fn an_agent_has_so_much_time_to_answer() {
    let (_held, lines) = sync_channel(1);

    let followed = follow(
        Some(&lines),
        Instant::now() + Duration::from_millis(60),
        &Cancellation::new(),
        &mut |_| {},
    );

    assert!(matches!(followed, Followed::UntilTooLate));
}

#[test]
fn two_calls_never_carry_the_same_mark() {
    assert_ne!(mark(), mark());
    assert_eq!(mark().len(), 32);
}
