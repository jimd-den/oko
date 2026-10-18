//! Typed, versioned IPC messages (SRD §4.2).
//!
//! Every IPC message is typed, versioned and validated. The envelope carries
//! a protocol version so the host and desktop UI can evolve independently.

use crate::ipc::json::{object, JsonValue};

/// IPC protocol version. Bumped on breaking message-format changes.
pub const IPC_VERSION: u64 = 1;

/// Direction of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Request,
    Response,
    Notification,
}

/// A routed IPC message envelope.
#[derive(Debug, Clone)]
pub struct Message {
    pub version: u64,
    pub id: String,
    pub direction: Direction,
    pub route: String, // e.g. "host.terminal.create", "ui.reload"
    pub payload: JsonValue,
    pub error: Option<String>,
}

impl Message {
    pub fn request(id: &str, route: &str, payload: JsonValue) -> Self {
        Self {
            version: IPC_VERSION,
            id: id.into(),
            direction: Direction::Request,
            route: route.into(),
            payload,
            error: None,
        }
    }

    pub fn response(id: &str, route: &str, payload: JsonValue) -> Self {
        Self {
            version: IPC_VERSION,
            id: id.into(),
            direction: Direction::Response,
            route: route.into(),
            payload,
            error: None,
        }
    }

    pub fn error_response(id: &str, route: &str, code: &str, message: &str) -> Self {
        Self {
            version: IPC_VERSION,
            id: id.into(),
            direction: Direction::Response,
            route: route.into(),
            payload: object(vec![
                ("code".into(), jstr(code)),
                ("message".into(), jstr(message)),
            ]),
            error: Some(format!("{}: {}", code, message)),
        }
    }

    pub fn notification(route: &str, payload: JsonValue) -> Self {
        Self {
            version: IPC_VERSION,
            id: "notify".into(),
            direction: Direction::Notification,
            route: route.into(),
            payload,
            error: None,
        }
    }
}

mod codec;

#[cfg(test)]
mod tests;

fn jstr(v: &str) -> JsonValue {
    JsonValue::String(v.into())
}
