//! An in-memory IPC transport.
//!
//! The stdio transport reads a real terminal, so it cannot be driven from a
//! test. This double lets a test script the host's incoming messages and read
//! back every message the host sent, so the whole message loop runs in-process.
//!
//! Hazard: `recv` returns "eof" once the script is exhausted, which is what
//! stops the host loop. Always end a script with `host.shutdown` or let it run
//! out; a test that appends to the script afterwards will hang.

use oko_core::ipc::json::JsonValue;
use oko_core::ipc::message::Message;
use oko_core::ipc::transport::IpcTransport;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Script {
    incoming: VecDeque<Message>,
    sent: Vec<Message>,
}

#[derive(Clone, Default)]
pub struct ScriptedTransport {
    script: Arc<Mutex<Script>>,
}

impl ScriptedTransport {
    pub fn new(incoming: Vec<Message>) -> Self {
        let transport = Self::default();
        transport.script.lock().unwrap().incoming = incoming.into();
        transport
    }

    /// The payload of the host's successful reply to `route`.
    pub fn reply_for(&self, route: &str) -> Option<JsonValue> {
        let reply_route = format!("{}.reply", route);
        self.script
            .lock()
            .unwrap()
            .sent
            .iter()
            .find(|m| m.route == reply_route && m.error.is_none())
            .map(|m| m.payload.clone())
    }

    /// The error text of the host's failed reply to `route`, if it failed.
    pub fn error_for(&self, route: &str) -> Option<String> {
        self.script
            .lock()
            .unwrap()
            .sent
            .iter()
            .find(|m| m.route == route && m.error.is_some())
            .and_then(|m| m.error.clone())
    }
}

impl IpcTransport for ScriptedTransport {
    fn send(&self, msg: &Message) -> Result<(), String> {
        self.script.lock().unwrap().sent.push(msg.clone());
        Ok(())
    }

    fn recv(&self) -> Result<Message, String> {
        self.script
            .lock()
            .unwrap()
            .incoming
            .pop_front()
            .ok_or_else(|| "eof".into())
    }
}
