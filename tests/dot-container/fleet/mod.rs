use super::*;
use crate::engine::tests::Scratch;

/// An engine whose one container lives in the files `state` and `fingerprint`.
pub(crate) const FAKE: &str = r#"
case "$1" in
  inspect)
    [ -f "$HERE/state" ] || { echo 'Error: No such container' >&2; exit 1; }
    printf '%s\t%s\n' "$(cat "$HERE/state")" "$(cat "$HERE/fingerprint")" ;;
  image) [ -f "$HERE/built" ] || { echo 'Error: No such image' >&2; exit 1; } ;;
  create) echo created > "$HERE/state"; echo "${5#*=}" > "$HERE/fingerprint" ;;
  start) echo running > "$HERE/state" ;;
  exec) . "$HERE/exec" ;;
esac
"#;

pub(crate) struct Fake {
    pub(crate) scratch: Scratch,
    pub(crate) fleet: Fleet,
    pub(crate) project: Project,
    pub(crate) name: AgentName,
    pub(crate) spec: AgentSpec,
    _folder: Scratch,
}

impl Fake {
    pub(crate) fn new(name: &str, exec: &str) -> Self {
        let scratch = Scratch::new(name);
        let folder = Scratch::new("project");
        scratch.write("built", "");
        scratch.write("exec", exec);
        Self {
            fleet: Fleet::new(scratch.engine(FAKE)),
            project: Project::new(folder.path(), None).unwrap(),
            name: AgentName::parse("reviewer").unwrap(),
            spec: AgentSpec::default(),
            scratch,
            _folder: folder,
        }
    }

    pub(crate) fn start(&self) -> Result<String, CallError> {
        self.fleet.start(
            (&self.name, &self.spec),
            &self.project,
            &Cancellation::new(),
        )
    }

    pub(crate) fn runs_of(&self, command: &str) -> usize {
        self.scratch
            .read("log")
            .lines()
            .filter(|line| line.starts_with(command))
            .count()
    }
}

#[test]
fn the_first_call_gives_the_agent_a_container_and_the_next_ones_find_it() {
    let fake = Fake::new("fleet-start", "");

    let first = fake.start().unwrap();
    let second = fake.start().unwrap();

    assert_eq!(first, second);
    assert_eq!(fake.runs_of("create"), 1);
    assert_eq!(fake.runs_of("start"), 1);
}

#[test]
fn wakes_an_agent_that_sleeps() {
    let fake = Fake::new("fleet-wake", "");
    fake.start().unwrap();
    fake.scratch.write("state", "exited");

    fake.start().unwrap();

    assert_eq!(fake.runs_of("create"), 1);
    assert_eq!(fake.runs_of("start"), 2);
}

#[test]
fn never_replaces_a_container_made_from_another_description() {
    let mut fake = Fake::new("fleet-outdated", "");
    fake.start().unwrap();
    fake.spec.memory = "1g".to_owned();

    let refused = fake.start().unwrap_err();

    assert!(matches!(refused, CallError::Outdated(_)));
    assert_eq!(fake.runs_of("rm"), 0);
}

#[test]
fn asks_for_the_image_to_be_built_first() {
    let fake = Fake::new("fleet-image", "");
    std::fs::rename(
        fake.scratch.path().join("built"),
        fake.scratch.path().join("gone"),
    )
    .unwrap();

    assert!(matches!(fake.start(), Err(CallError::NotBuilt)));
    assert_eq!(fake.runs_of("create"), 0);
}

#[test]
fn an_engine_that_does_not_answer_is_no_absent_container() {
    let scratch = Scratch::new("fleet-down");
    let folder = Scratch::new("project");
    let fleet = Fleet::new(
        scratch
            .engine("echo 'dial unix /var/run/docker.sock: no such file or directory' >&2; exit 1"),
    );
    let project = Project::new(folder.path(), None).unwrap();
    let name = AgentName::parse("reviewer").unwrap();

    let refused = fleet.start(
        (&name, &AgentSpec::default()),
        &project,
        &Cancellation::new(),
    );

    assert!(matches!(
        refused,
        Err(CallError::Engine(EngineError::Refused(_)))
    ));
    assert!(!scratch.read("log").contains("create"));
}

#[test]
fn lists_the_containers_of_the_fleet() {
    let scratch = Scratch::new("fleet-list");
    let fleet = Fleet::new(
        scratch
            .engine("printf 'dot-reviewer-1\\trunning\\treviewer\\t/home/me/project\\nbroken\\n'"),
    );

    let listed = fleet.list(&Cancellation::new()).unwrap();

    assert_eq!(
        listed,
        [Listed {
            container: "dot-reviewer-1".to_owned(),
            state: "running".to_owned(),
            agent: "reviewer".to_owned(),
            folder: "/home/me/project".to_owned(),
        }]
    );
}
