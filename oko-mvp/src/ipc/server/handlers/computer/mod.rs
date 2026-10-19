//! Handler: computer.ask — one request, read the way a computer reads it.
//!
//! The interpretation is a pure domain function (`domain::computer`); the
//! wording of an answer lives in `replies`; this file is the flow between them.

use crate::domain::computer::{interpret, Intent};
use crate::domain::session::SessionId;
use crate::ipc::json::object;
use crate::ipc::server::Core;

mod replies;

#[cfg(test)]
mod tests;

use replies::{engine_reply, open_reply, run_reply, status_reply};

/// The session a request runs in when the caller does not name one. Reused, so
/// consecutive requests share a shell the way one conversation should.
const DEFAULT_SESSION: &str = "computer";

pub fn handle_ask(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let text = payload_string(payload, "text")?;
    let requested = payload_string(payload, "session_id").ok();

    let (reply, session) = match interpret(&text) {
        Intent::Nothing => ("Ready.".to_string(), None),
        Intent::Identify => (
            "I am ỌKỌ, a terminal-first computer. Ask for status, or type a command.".to_string(),
            None,
        ),
        Intent::Status => (status_reply(server), None),
        Intent::Engine => (engine_reply(server), None),
        Intent::Open(url) => (open_reply(server, &url)?, None),
        Intent::Run(command) => {
            let id = session_for(server, requested)?;
            (run_reply(server, &id, &command)?, Some(id.0))
        }
    };

    let mut fields = vec![("reply".into(), str(&reply))];
    if let Some(id) = session {
        fields.push(("session_id".into(), str(&id)));
    }
    Ok(object(fields))
}

/// The session this request runs in, created on first use.
fn session_for(server: &mut Core, requested: Option<String>) -> Result<SessionId, String> {
    let id = SessionId(requested.unwrap_or_else(|| DEFAULT_SESSION.to_string()));
    if server.sessions.pty(&id).is_none() {
        server
            .sessions
            .create_terminal(server.terminal.as_ref(), id.clone())
            .map_err(|e| format!("{:?}", e))?;
    }
    Ok(id)
}

pub(super) fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

pub(super) fn payload_string(
    payload: &crate::ipc::json::JsonValue,
    key: &str,
) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}
