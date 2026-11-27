use std::path::PathBuf;

use dot_api::{Arrangement, Hue, KeptId, KeptSession};
use serde::Serialize;
use serde_json::Value;

use crate::readable::is_readable;
use crate::settings::{hue_name, hue_named};

pub(crate) const INDEX: &str = "index.json";
const INDEX_FORMAT: u64 = 1;
const FORMAT: &str = "format";
const SHOWN: &str = "shown";
const SESSIONS: &str = "sessions";
const ID: &str = "id";
const NAME: &str = "name";
const COLOR: &str = "color";
const FOLDER: &str = "folder";
const LONGEST_NAME: usize = 64;
const UNKNOWN_COLOR: Hue = Hue::Violet;

#[derive(Serialize)]
struct Index<'a> {
    format: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    shown: Option<String>,
    sessions: Vec<Entry<'a>>,
}

#[derive(Serialize)]
struct Entry<'a> {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    color: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<&'a str>,
}

pub(crate) fn written(arrangement: &Arrangement) -> serde_json::Result<Vec<u8>> {
    let sessions: Vec<Entry<'_>> = arrangement.sessions.iter().filter_map(entry_of).collect();
    let shown = arrangement
        .shown
        .filter(|shown| sessions.iter().any(|entry| entry.id == shown.to_string()))
        .map(|shown| shown.to_string());
    let index = Index {
        format: INDEX_FORMAT,
        shown,
        sessions,
    };
    serde_json::to_vec(&index)
}

pub(crate) fn read(written: &[u8]) -> Option<Arrangement> {
    let index = serde_json::from_slice::<Value>(written).ok()?;
    if index.get(FORMAT).and_then(Value::as_u64) != Some(INDEX_FORMAT) {
        return None;
    }

    let mut sessions: Vec<KeptSession> = Vec::new();
    for entry in index
        .get(SESSIONS)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(session) = session_of(entry) else {
            continue;
        };
        if sessions.iter().all(|kept| kept.id != session.id) {
            sessions.push(session);
        }
    }
    let shown = index
        .get(SHOWN)
        .and_then(Value::as_str)
        .and_then(KeptId::parse)
        .filter(|shown| sessions.iter().any(|kept| kept.id == *shown));
    Some(Arrangement { sessions, shown })
}

fn entry_of(session: &KeptSession) -> Option<Entry<'_>> {
    let folder = match &session.folder {
        Some(folder) => Some(folder.to_str()?),
        None => None,
    };
    Some(Entry {
        id: session.id.to_string(),
        name: session.name.as_deref(),
        color: hue_name(session.hue),
        folder,
    })
}

fn session_of(entry: &Value) -> Option<KeptSession> {
    let id = KeptId::parse(entry.get(ID)?.as_str()?)?;
    let name = entry
        .get(NAME)
        .and_then(Value::as_str)
        .filter(|name| is_a_name(name))
        .map(str::to_owned);
    let hue = entry
        .get(COLOR)
        .and_then(Value::as_str)
        .and_then(hue_named)
        .unwrap_or(UNKNOWN_COLOR);
    let folder = entry
        .get(FOLDER)
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|folder| folder.is_absolute());
    Some(KeptSession {
        id,
        name,
        hue,
        folder,
    })
}

fn is_a_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= LONGEST_NAME && name.chars().all(is_readable)
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/archive/index.rs"]
mod tests;
