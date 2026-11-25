use crate::engine::EngineKind;
use crate::fleet::project::Project;
use crate::spec::{Access, AgentName, AgentSpec, Network};

pub(crate) const AGENT_LABEL: &str = "dev.dot.agent";
pub(crate) const FOLDER_LABEL: &str = "dev.dot.folder";
pub(crate) const FINGERPRINT_LABEL: &str = "dev.dot.fingerprint";
pub(crate) const CALL_VARIABLE: &str = "DOT_CALL";
const HOME: &str = "/home/agent";
const CLAUDE: &str = "claude";
const STAY_ALIVE: [&str; 2] = ["sleep", "infinity"];
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Ends what a call started: every process of the container that carries its mark. A process
/// cannot lose it by leaving its group, and the processes of other calls carry another. It is
/// asked to end, then ended, by one command, which goes on when whoever ran it is gone.
const END_CALL: &str = r#"command -v tr >/dev/null && command -v grep >/dev/null || exit 1
signal() {
  found=1
  for process in /proc/[0-9]*; do
    if tr '\0' '\n' 2>/dev/null < "$process/environ" | grep -qx "DOT_CALL=$1"; then
      found=0
      kill -s "$2" "${process#/proc/}"
    fi
  done
  return "$found"
}
if signal "$1" TERM; then
  sleep 1
  signal "$1" KILL
fi
exit 0"#;

/// Who the container runs as: the user, so that what it writes in the project is theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Owner {
    pub(crate) user: u32,
    pub(crate) group: u32,
}

pub(crate) fn container_name(name: &AgentName, project: &Project) -> String {
    format!("dot-{name}-{:016x}", hash(project.text().as_bytes()))
}

pub(crate) fn create(
    container: &str,
    (name, spec): (&AgentName, &AgentSpec),
    project: &Project,
    (kind, owner): (EngineKind, Owner),
) -> Vec<String> {
    let settings = settings((name, spec), project, (kind, owner));
    let fingerprint = fingerprint(&settings);
    let mut arguments = strings(&["create", "--name", container]);
    arguments.extend(strings(&["--label"]));
    arguments.push(format!("{FINGERPRINT_LABEL}={fingerprint}"));
    arguments.extend(settings);
    arguments
}

/// The mark of everything a container was created with: another one means another container.
pub(crate) fn fingerprint_of(
    agent: (&AgentName, &AgentSpec),
    project: &Project,
    run: (EngineKind, Owner),
) -> String {
    fingerprint(&settings(agent, project, run))
}

pub(crate) fn call(
    container: &str,
    spec: &AgentSpec,
    project: &Project,
    (mark, goes_on): (&str, bool),
) -> Vec<String> {
    let mut arguments = strings(&["exec", "--interactive"]);
    for variable in &spec.env {
        arguments.extend(strings(&["--env", variable]));
    }
    arguments.extend(strings(&["--env", &format!("{CALL_VARIABLE}={mark}")]));
    arguments.extend(strings(&["--workdir", &workdir(spec, project), container]));
    if spec.command.is_empty() {
        arguments.extend(claude(spec, goes_on));
    } else {
        arguments.extend(spec.command.iter().cloned());
    }
    arguments
}

pub(crate) fn end_call(container: &str, mark: &str) -> Vec<String> {
    strings(&[
        "exec",
        container,
        "sh",
        "-c",
        END_CALL,
        "dot-end-call",
        mark,
    ])
}

pub(crate) fn stop(container: &str) -> Vec<String> {
    strings(&["stop", "--time", "2", container])
}

pub(crate) fn sign_in(container: &str) -> Vec<String> {
    strings(&[
        "exec",
        "--interactive",
        "--tty",
        container,
        CLAUDE,
        "auth",
        "login",
    ])
}

fn settings(
    (name, spec): (&AgentName, &AgentSpec),
    project: &Project,
    (kind, owner): (EngineKind, Owner),
) -> Vec<String> {
    let folder = project.text();
    let mut arguments = strings(&["--init", "--cap-drop", "ALL"]);
    arguments.extend(strings(&["--security-opt", "no-new-privileges"]));
    arguments.extend(strings(&["--memory", &spec.memory]));
    arguments.extend(strings(&["--memory-swap", &spec.memory]));
    arguments.extend(strings(&["--cpus", &spec.cpus.to_string()]));
    arguments.extend(strings(&["--pids-limit", &spec.processes.to_string()]));
    arguments.push("--user".to_owned());
    arguments.push(format!("{}:{}", owner.user, owner.group));
    if kind == EngineKind::Podman {
        arguments.push("--userns=keep-id".to_owned());
    }
    if spec.network == Network::None {
        arguments.extend(strings(&["--network", "none"]));
    }
    for (label, value) in [(AGENT_LABEL, name.as_str()), (FOLDER_LABEL, &folder)] {
        arguments.push("--label".to_owned());
        arguments.push(format!("{label}={value}"));
    }
    arguments.push("--env".to_owned());
    arguments.push(format!("HOME={HOME}"));
    arguments.push("--mount".to_owned());
    arguments.push(format!("type=volume,source=dot-agent-{name},target={HOME}"));
    match spec.workspace {
        Access::None => {}
        Access::Read => arguments.extend(mount(&folder, ",readonly")),
        Access::Write => arguments.extend(mount(&folder, "")),
    }
    arguments.push("--entrypoint".to_owned());
    arguments.push(String::new());
    arguments.push(spec.image.clone());
    arguments.extend(strings(&STAY_ALIVE));
    arguments
}

fn mount(folder: &str, mode: &str) -> [String; 2] {
    [
        "--mount".to_owned(),
        format!("type=bind,source={folder},target={folder}{mode}"),
    ]
}

/// The project lies at the same path inside as outside: the paths an agent names are the user's,
/// and Claude Code keeps the conversations of two projects apart.
fn workdir(spec: &AgentSpec, project: &Project) -> String {
    match spec.workspace {
        Access::None => HOME.to_owned(),
        Access::Read | Access::Write => project.text(),
    }
}

/// Claude Code asks nobody inside: what it may touch is decided by the container.
fn claude(spec: &AgentSpec, goes_on: bool) -> Vec<String> {
    let mut arguments = strings(&[
        CLAUDE,
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "bypassPermissions",
    ]);
    if let Some(model) = &spec.model {
        arguments.extend(strings(&["--model", model]));
    }
    if !spec.instructions.is_empty() {
        arguments.extend(strings(&["--append-system-prompt", &spec.instructions]));
    }
    if goes_on {
        arguments.push("--continue".to_owned());
    }
    arguments
}

fn fingerprint(settings: &[String]) -> String {
    format!("{:016x}", hash(settings.join("\0").as_bytes()))
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn strings(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|&argument| argument.to_owned())
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/fleet/arguments.rs"]
mod tests;
