use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use dot_api::{
    Action, Cancellation, Decision, Detail, Permission, Permissions, ToolCall, ToolOutput, Toolbox,
};

pub(crate) struct FakeToolbox {
    folder: Option<PathBuf>,
    calls: Mutex<Vec<(String, ToolCall)>>,
}

pub(crate) struct Answering {
    decision: Decision,
    asked: Mutex<Vec<Permission>>,
}

impl FakeToolbox {
    pub(crate) fn in_folder(folder: &str) -> Self {
        Self {
            folder: Some(PathBuf::from(folder)),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn without_folder() -> Self {
        Self {
            folder: None,
            calls: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn calls(&self) -> Vec<(String, ToolCall)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Toolbox for FakeToolbox {
    fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    fn holds(&self, path: &str) -> bool {
        self.folder
            .as_deref()
            .is_some_and(|folder| Path::new(path).starts_with(folder))
    }

    fn run(
        &self,
        call: &ToolCall,
        tool_id: &str,
        permissions: &dyn Permissions,
        cancellation: &Cancellation,
    ) -> ToolOutput {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((tool_id.to_owned(), call.clone()));
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
            return ToolOutput::done(format!("ran {command}"), Detail::Exit(0));
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
