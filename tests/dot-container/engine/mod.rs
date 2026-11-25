use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

static SCRATCHES: AtomicU32 = AtomicU32::new(0);

/// A folder of the test's own.
pub(crate) struct Scratch(PathBuf);

impl Scratch {
    pub(crate) fn new(name: &str) -> Self {
        let number = SCRATCHES.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let folder = std::env::temp_dir().join(format!("dot-container-{name}-{process}-{number}"));
        fs::create_dir_all(&folder).unwrap();
        Self(fs::canonicalize(folder).unwrap())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn write(&self, name: &str, text: &str) {
        fs::write(self.0.join(name), text).unwrap();
    }

    pub(crate) fn read(&self, name: &str) -> String {
        fs::read_to_string(self.0.join(name)).unwrap_or_default()
    }

    /// An engine that writes the arguments of each of its runs in `log`, then runs `script`.
    pub(crate) fn engine(&self, script: &str) -> Engine {
        let program = self.0.join("engine");
        let log = self.0.join("log");
        let text = format!(
            "#!/bin/sh\nHERE='{}'\nprintf '%s ' \"$@\" >> '{}'\necho >> '{}'\n{script}\n",
            self.0.display(),
            log.display(),
            log.display(),
        );
        self.write("script", &text);
        // Another process writes the program: a file this one holds open for writing, even in a
        // program another test is starting, cannot run.
        let installed = dot_process::command("install")
            .args(["-m", "755"])
            .args([&self.0.join("script"), &program])
            .status()
            .unwrap();
        assert!(installed.success());
        Engine::new(EngineKind::Docker, program)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

#[test]
fn tells_the_version_of_the_engine() {
    let scratch = Scratch::new("version");

    let version = scratch.engine("echo 29.0").version(&Cancellation::new());

    assert_eq!(version.unwrap(), "29.0");
}

#[test]
fn tells_why_the_engine_refuses() {
    let scratch = Scratch::new("refuses");
    let engine = scratch.engine("echo 'the daemon is not running' >&2; exit 1");

    let refused = engine.version(&Cancellation::new()).unwrap_err();

    assert!(refused.to_string().contains("the daemon is not running"));
}

#[test]
fn tells_what_docker_without_its_service_prints_on_its_output() {
    let scratch = Scratch::new("stand-in");
    let engine = scratch.engine("echo 'not found in this WSL 2 distro'; exit 1");

    let refused = engine.version(&Cancellation::new()).unwrap_err();

    assert!(
        refused
            .to_string()
            .contains("not found in this WSL 2 distro")
    );
}

#[test]
fn an_engine_that_cannot_start_is_an_error() {
    let engine = Engine::new(EngineKind::Docker, PathBuf::from("/nowhere/docker"));

    let error = engine.version(&Cancellation::new()).unwrap_err();

    assert!(matches!(error, EngineError::Start(..)));
}

#[test]
fn stops_waiting_for_an_engine_that_hangs() {
    let scratch = Scratch::new("hangs");
    let engine = scratch.engine("sleep 30");

    let late = engine.ask(["ps"], Duration::from_millis(100), &Cancellation::new());

    assert!(matches!(late, Err(EngineError::TimedOut(_))));
}

#[test]
fn stops_the_engine_when_asked_to() {
    let scratch = Scratch::new("cancelled");
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let stopped = scratch.engine("sleep 30").ask(["ps"], QUICK, &cancellation);

    assert!(matches!(stopped, Err(EngineError::Cancelled)));
}

#[test]
fn builds_the_image_from_the_recipe_it_embeds() {
    let scratch = Scratch::new("build");
    let engine = scratch.engine("cat > \"$HERE/recipe\"");

    engine
        .build("dot-agent:latest", &Cancellation::new())
        .unwrap();

    assert!(
        scratch
            .read("log")
            .contains("build --pull --tag dot-agent:latest -")
    );
    assert_eq!(scratch.read("recipe"), DOCKERFILE);
}

#[test]
fn prefers_docker_and_takes_podman_without_it() {
    let scratch = Scratch::new("locate");
    scratch.write("podman", "");
    let path = scratch.path().as_os_str().to_owned();
    let podman = locate_engine(Some(&path));
    scratch.write("docker", "");

    let docker = locate_engine(Some(&path)).unwrap();

    assert!(podman.is_some());
    assert_eq!(docker.kind(), EngineKind::Docker);
    assert_eq!(docker.program(), scratch.path().join("docker"));
}
