//! IPC transport abstraction and stdio implementation.
//!
//! The host and desktop UI exchange typed, versioned messages over a message
//! channel. `IpcTransport` is the port; stdio is the MVP adapter.

use crate::ipc::message::Message;
use std::io::Write;

/// A bidirectional message channel.
pub trait IpcTransport: Send + Sync {
    /// Send one message (serialized to a single line).
    fn send(&self, msg: &Message) -> Result<(), String>;
    /// Receive one message (blocks until a full line is available).
    fn recv(&self) -> Result<Message, String>;
}

/// Stdio transport: reads lines from stdin, writes lines to stdout.
pub struct StdioTransport;

impl Default for StdioTransport {
    fn default() -> Self {
        Self
    }
}

impl StdioTransport {
    pub fn new() -> Self {
        Self
    }
}

impl IpcTransport for StdioTransport {
    fn send(&self, msg: &Message) -> Result<(), String> {
        let line = msg.to_line();
        println!("{}", line);
        std::io::stdout().flush().map_err(|e| format!("flush: {}", e))
    }

    fn recv(&self) -> Result<Message, String> {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).map_err(|e| format!("read: {}", e))?;
        if line.is_empty() {
            return Err("eof".into());
        }
        let line = line.trim_end_matches(&['\n', '\r'][..]);
        Message::from_line(line)
    }
}

/// A message bus built on a transport. Handles request/response correlation.
pub struct MessageBus<T: IpcTransport> {
    transport: T,
}

impl<T: IpcTransport> MessageBus<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn send(&self, msg: &Message) -> Result<(), String> {
        self.transport.send(msg)
    }

    pub fn recv(&self) -> Result<Message, String> {
        self.transport.recv()
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_serialization_is_stable() {
        let m = Message::request("1", "host.status", crate::ipc::json::object(vec![]));
        let line = m.to_line();
        assert!(line.contains("\"route\":\"host.status\""));
        assert!(line.contains("\"v\":1"));
        assert!(Message::from_line(&line).is_ok());
    }
}
