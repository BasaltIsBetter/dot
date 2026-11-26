//! Agents that live in containers and answer when they are called.
//!
//! Each agent is described by one JSON file of the user's, an [`AgentSpec`] of the [`Roster`]:
//! its image, what it may touch of the project, its limits, its instructions. The [`Fleet`] gives
//! it a container of its own for each project, which stays alive between calls: the first call
//! creates it, the next ones start from there. [`Fleet::call`] hands a prompt to the agent and
//! returns its answer, with what it does meanwhile; stopping a call ends what it started inside
//! the container too.
//!
//! The [`Engine`] is the user's own Docker or Podman, through its command line. A container has
//! no capability, gains no privilege, runs as the user, and sees of this machine only the
//! project's folder, read-only unless the agent's file says otherwise. Agents are never read from
//! a project: opening one must not decide which images run or what they may see.
//!
//! [`serve`] offers the same calls to Claude over the Model Context Protocol; the `dot-agents`
//! program offers them to the user in a terminal.

mod engine;
mod fleet;
mod mcp;
mod roster;
mod spec;

pub use crate::engine::{Engine, EngineError, EngineKind, locate_engine};
pub use crate::fleet::{Answer, Call, CallError, CallEvent, Fleet, Listed, Project, ProjectError};
pub use crate::mcp::serve;
pub use crate::roster::{Roster, RosterError, agents_folder, has_agents};
pub use crate::spec::{Access, AgentName, AgentSpec, Network, SpecError};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
