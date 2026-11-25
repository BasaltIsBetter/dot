use super::*;
use crate::engine::tests::Scratch;

const OWNER: Owner = Owner {
    user: 1000,
    group: 1001,
};
const DOCKER: (EngineKind, Owner) = (EngineKind::Docker, OWNER);

fn name() -> AgentName {
    AgentName::parse("reviewer").unwrap()
}

fn project(scratch: &Scratch) -> Project {
    Project::new(scratch.path(), None).unwrap()
}

fn created(spec: &AgentSpec, project: &Project, run: (EngineKind, Owner)) -> String {
    create("box", (&name(), spec), project, run).join(" ")
}

#[test]
fn a_container_is_locked_down() {
    let scratch = Scratch::new("arguments-locked");
    let created = created(&AgentSpec::default(), &project(&scratch), DOCKER);

    for lock in [
        "--cap-drop ALL",
        "--security-opt no-new-privileges",
        "--memory 2g",
        "--memory-swap 2g",
        "--cpus 2",
        "--pids-limit 512",
        "--user 1000:1001",
    ] {
        assert!(created.contains(lock), "{lock}: {created}");
    }
    assert!(!created.contains("--privileged"));
    assert!(!created.contains("docker.sock"));
}

#[test]
fn the_project_is_seen_as_much_as_described_at_its_own_path() {
    let scratch = Scratch::new("arguments-mount");
    let project = project(&scratch);
    let folder = project.text();
    let with = |workspace| AgentSpec {
        workspace,
        ..AgentSpec::default()
    };
    let mount = format!("type=bind,source={folder},target={folder}");

    let read = created(&with(Access::Read), &project, DOCKER);
    let write = created(&with(Access::Write), &project, DOCKER);
    let none = created(&with(Access::None), &project, DOCKER);

    assert!(read.contains(&format!("{mount},readonly")));
    assert!(write.contains(&mount) && !write.contains("readonly"));
    assert!(!none.contains("type=bind"));
}

#[test]
fn an_agent_without_network_has_none() {
    let scratch = Scratch::new("arguments-network");
    let spec = AgentSpec {
        network: Network::None,
        ..AgentSpec::default()
    };

    assert!(created(&spec, &project(&scratch), DOCKER).contains("--network none"));
}

#[test]
fn podman_keeps_the_identity_of_the_user() {
    let scratch = Scratch::new("arguments-podman");
    let project = project(&scratch);
    let spec = AgentSpec::default();

    assert!(created(&spec, &project, (EngineKind::Podman, OWNER)).contains("--userns=keep-id"));
    assert!(!created(&spec, &project, DOCKER).contains("--userns"));
}

#[test]
fn another_description_makes_another_fingerprint() {
    let scratch = Scratch::new("arguments-fingerprint");
    let project = project(&scratch);
    let spec = AgentSpec::default();
    let wider = AgentSpec {
        workspace: Access::Write,
        ..AgentSpec::default()
    };
    let fingerprint = |spec| fingerprint_of((&name(), spec), &project, DOCKER);

    assert_eq!(fingerprint(&spec), fingerprint(&spec));
    assert_ne!(fingerprint(&spec), fingerprint(&wider));
    assert!(created(&spec, &project, DOCKER).contains(&fingerprint(&spec)));
}

#[test]
fn each_project_has_a_container_of_its_own() {
    let one = Scratch::new("arguments-one");
    let other = Scratch::new("arguments-other");

    assert_ne!(
        container_name(&name(), &project(&one)),
        container_name(&name(), &project(&other))
    );
}

#[test]
fn a_hash_never_changes_from_one_version_to_the_next() {
    assert_eq!(hash(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(hash(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn claude_code_answers_unless_the_agent_has_a_program() {
    let scratch = Scratch::new("arguments-call");
    let project = project(&scratch);
    let claude = AgentSpec {
        model: Some("haiku".to_owned()),
        instructions: "Review.".to_owned(),
        env: vec!["TOKEN".to_owned()],
        ..AgentSpec::default()
    };
    let program = AgentSpec {
        command: vec!["wc".to_owned(), "-w".to_owned()],
        ..AgentSpec::default()
    };

    let asked = call("box", &claude, &project, ("m4rk", true)).join(" ");
    let ran = call("box", &program, &project, ("m4rk", true)).join(" ");

    assert!(asked.starts_with("exec --interactive --env TOKEN --env DOT_CALL=m4rk --workdir "));
    assert!(asked.contains("box claude -p --output-format stream-json"));
    assert!(asked.ends_with("--model haiku --append-system-prompt Review. --continue"));
    assert!(ran.ends_with("box wc -w"));
}

#[test]
fn ends_every_process_of_a_call_and_no_other() {
    let of_the_call = || {
        let mut command = dot_process::command("sleep");
        command
            .arg("30")
            .env(CALL_VARIABLE, "m4rk")
            .spawn()
            .unwrap()
    };
    if !cfg!(target_os = "linux") {
        return;
    }
    let mut first = of_the_call();
    let mut second = of_the_call();
    let mut other = dot_process::command("sleep")
        .arg("30")
        .env(CALL_VARIABLE, "m4rk2")
        .spawn()
        .unwrap();

    let arguments = end_call("box", "m4rk", "KILL");
    let status = dot_process::command(&arguments[2])
        .args(&arguments[3..])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(!first.wait().unwrap().success());
    assert!(!second.wait().unwrap().success());
    assert!(other.try_wait().unwrap().is_none());
    dot_process::stop(&mut other).unwrap();
}
