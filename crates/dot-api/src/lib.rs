//! Contracts and domain types shared by every Dot crate.

mod api_key;
mod connection;
mod text;

pub use crate::api_key::{ApiKey, ApiKeyError};
pub use crate::connection::{
    Cancellation, Cause, ConnectError, ConnectFailure, Connection, Connector,
};
pub use crate::text::{Text, Translator};
