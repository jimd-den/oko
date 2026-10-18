//! The wire format: one line of JSON in, one line of JSON out.
//!
//! Kept apart from the message type so the envelope's shape and its encoding
//! can change for different reasons, and so the version check that rejects a
//! foreign protocol version sits next to the parser that can fail on it.

use super::{Direction, Message, IPC_VERSION};
use crate::ipc::json::JsonValue;

impl Message {
    /// Serialize to one line of JSON for the transport.
    pub fn to_line(&self) -> String {
        let dir = match self.direction {
            Direction::Request => "request",
            Direction::Response => "response",
            Direction::Notification => "notification",
        };
        let payload = crate::ipc::json::stringify(&self.payload);
        let id_json =
            crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(self.id.clone()));
        let dir_json =
            crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(dir.into()));
        let route_json =
            crate::ipc::json::stringify(&crate::ipc::json::JsonValue::String(self.route.clone()));
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
            _ => {
                return Err(format!(
                    "unsupported IPC version (expected {})",
                    IPC_VERSION
                ))
            }
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
        Ok(Self {
            version,
            id,
            direction,
            route,
            payload,
            error,
        })
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
