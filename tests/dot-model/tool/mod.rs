use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use dot_api::{
    Action, Cancellation, Compactor, Decision, Detail, FileChange, Origin, OutputCompaction,
    Permission, Permissions, ToolCall, ToolOutput, Toolbox,
};

pub(crate) struct FakeToolbox {
    folder: Option<PathBuf>,
    calls: Vec<(String, ToolCall)>,
}

pub(crate) struct Untouched;

pub(crate) struct Naming;

impl Compactor for Untouched {
    fn compact(&self, _: Origin<'_>, _: &str) -> Option<String> {
        None
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(output)
    }

    fn tune(&mut self, _: OutputCompaction) {}
}

impl Compactor for Naming {
    fn compact(&self, origin: Origin<'_>, output: &str) -> Option<String> {
        Some(format!("{origin:?} of {} bytes", output.len()))
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        Cow::Owned(output.replace('\u{1b}', ""))
    }

    fn tune(&mut self, _: OutputCompaction) {}
}

pub(crate) struct Answering {
    decision: Decision,
    asked: Mutex<Vec<Permission>>,
}

impl FakeToolbox {
    pub(crate) fn in_folder(folder: &str) -> Self {
        Self {
            folder: Some(PathBuf::from(folder)),
            calls: Vec::new(),
        }
    }

    pub(crate) fn without_folder() -> Self {
        Self {
            folder: None,
            calls: Vec::new(),
        }
    }

    pub(crate) fn calls(&self) -> Vec<(String, ToolCall)> {
        self.calls.clone()
    }
}

impl Toolbox for FakeToolbox {
    fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    fn inside(&self, path: &str) -> Option<String> {
        let folder = self.folder.as_deref()?;
        let inside = Path::new(path).strip_prefix(folder).ok()?;
        Some(inside.display().to_string())
    }

    fn run(
        &mut self,
        call: &ToolCall,
        tool_id: &str,
        permissions: &dyn Permissions,
        cancellation: &Cancellation,
        on_output: &mut dyn FnMut(&[u8]),
    ) -> ToolOutput {
        self.calls.push((tool_id.to_owned(), call.clone()));
        if let ToolCall::Run { command } = call {
            let permission = Permission {
                tool_id: tool_id.to_owned(),
                action: Action::Run {
                    command: command.clone(),
                },
            };
            if permissions.ask(&permission, cancellation) == Decision::Deny {
                return ToolOutput::refused();
            }
            on_output(format!("\u{1b}[1mrunning\u{1b}[0m {command}\n").as_bytes());
            return ToolOutput::done(format!("ran {command}"), Detail::Exit(0));
        }
        if let ToolCall::Create { path, text } = call {
            let change = FileChange::between(path, "", text, true);
            return ToolOutput::done(
                "written",
                Detail::Changes {
                    added: 1,
                    removed: 0,
                },
            )
            .changing(change);
        }
        ToolOutput::done("seen", Detail::Lines(1))
    }
}

impl Answering {
    pub(crate) fn with(decision: Decision) -> Self {
        Self {
            decision,
            asked: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn asked(&self) -> Vec<Permission> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Permissions for Answering {
    fn ask(&self, permission: &Permission, _cancellation: &Cancellation) -> Decision {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(permission.clone());
        self.decision
    }
}
