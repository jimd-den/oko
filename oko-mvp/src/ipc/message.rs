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

    /// Serialize to one line of JSON for the transport.
    pub fn to_line(&self) -> String {
        let dir = match self.direction {
            Direction::Request => "request",
            Direction::Response => "response",
            Direction::Notification => "notification",
        };
        let payload = crate::ipc::json::stringify(&self.payload);
        let id_json = crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(self.id.clone()));
        let dir_json = crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(dir.into()));
        let route_json = crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(self.route.clone()));
        let error = match &self.error {
            Some(e) => format!(
                ", \"error\": {}",
                crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(e.clone()))
            ),
            None => "".into(),
        };
        format!(
            "{{\"v\":{},\"id\":{},\"direction\":{},\"route\":{},\"payload\":{}{}}}",
            self.version, id_json, dir_json, route_json, payload, error
        )
    }

    /// Parse one line of JSON into a Message, validating the envelope.
    pub fn from_line(line: &str) -> Result<Self, String> {
        let v = crate::ipc::json::parse(line).map_err(|e| format!("json parse: {:?}", e.0))?;
        let version = match json_get_num(&v, "v") {
            Some(n) if n == IPC_VERSION as f64 => n as u64,
            _ => return Err(format!("unsupported IPC version (expected {})", IPC_VERSION)),
        };
        let id = json_get_str(&v, "id").ok_or("missing id")?.into();
        let direction = match json_get_str(&v, "direction").ok_or("missing direction")? {
            "request" => Direction::Request,
            "response" => Direction::Response,
            "notification" => Direction::Notification,
            d => return Err(format!("unknown direction: {}", d)),
        };
        let route = json_get_str(&v, "route").ok_or("missing route")?.into();
        let payload = json_get(&v, "payload").cloned().unwrap_or(JsonValue::Null);
        let error = json_get(&v, "error").and_then(|e| match e {
            JsonValue::Null => None,
            JsonValue::String(s) => Some(s.clone()),
            _ => None,
        });
        Ok(Self { version, id, direction, route, payload, error })
    }
}

fn json_get_num(obj: &JsonValue, key: &str) -> Option<f64> {
    crate::ipc::json::get(obj, key).and_then(crate::ipc::json::as_num)
}

fn json_get_str<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a str> {
    crate::ipc::json::get(obj, key).and_then(crate::ipc::json::as_str)
}

fn json_get<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    crate::ipc::json::get(obj, key)
}

fn jstr(v: &str) -> JsonValue {
    JsonValue::String(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let m = Message::request("1", "host.terminal.create", object(vec![]));
        let line = m.to_line();
        let back = Message::from_line(&line).unwrap();
        assert_eq!(back.version, 1);
        assert_eq!(back.id, "1");
        assert_eq!(back.route, "host.terminal.create");
    }

    #[test]
    fn error_response_roundtrip() {
        let m = Message::error_response("2", "host.app.launch", "permission_denied", "missing process.spawn");
        let line = m.to_line();
        let back = Message::from_line(&line).unwrap();
        assert!(back.error.is_some());
        assert_eq!(back.route, "host.app.launch");
    }

    #[test]
    fn rejects_wrong_version() {
        let mut m = Message::request("1", "host.status", object(vec![]));
        m.version = 999;
        let line = m.to_line();
        assert!(Message::from_line(&line).is_err());
    }

    #[test]
    fn notification() {
        let m = Message::notification("ui.reload", object(vec![]));
        assert!(matches!(m.direction, Direction::Notification));
        let back = Message::from_line(&m.to_line()).unwrap();
        assert_eq!(back.route, "ui.reload");
    }
}
