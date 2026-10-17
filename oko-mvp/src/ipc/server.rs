//! IPC server — wires host API, terminal sessions, command service, process
//! adapter and mock browser engine behind the message bus (SRD §4.2).

use crate::application::session_service::SessionService;
use crate::browser::ports::engine::{BrowserConfig, BrowserEnginePort};
use crate::domain::session::SessionId;
use crate::ipc::json::object;
use crate::ipc::message::{Message, IPC_VERSION};
use crate::ipc::transport::{IpcTransport, MessageBus};
use crate::ports::process::{ProcessPort, SpawnRequest};
use crate::ports::terminal::TerminalPort;

/// The IPC server: one message loop, routes to host API / core services.
pub struct IpcServer<T: IpcTransport> {
    bus: MessageBus<T>,
    process: Box<dyn ProcessPort>,
    terminal: Box<dyn TerminalPort>,
    sessions: SessionService,
    browser: Box<dyn BrowserEnginePort>,
}

impl IpcServer<crate::ipc::transport::StdioTransport> {
    pub fn new(
        process: Box<dyn ProcessPort>,
        terminal: Box<dyn TerminalPort>,
        sessions: SessionService,
        browser: Box<dyn BrowserEnginePort>,
    ) -> Self {
        Self {
            bus: MessageBus::new(crate::ipc::transport::StdioTransport::new()),
            process,
            terminal,
            sessions,
            browser,
        }
    }

    /// Run the message loop until EOF or a shutdown request.
    pub fn run(&mut self) -> Result<(), String> {
        self.browser.initialize().map_err(|e| format!("{:?}", e))?;
        loop {
            let msg = match self.bus.recv() {
                Ok(m) => m,
                Err(e) if e == "eof" => break,
                Err(e) => {
                    let _ = self.bus.send(&Message::error_response("0", "host.error", "io", &e));
                    continue;
                }
            };

            if msg.route == "host.shutdown" {
                let _ = self.bus.send(&Message::response(&msg.id, &msg.route, object(vec![])));
                break;
            }

            let response = self.handle_message(&msg);
            let _ = self.bus.send(&response);
        }
        self.browser.shutdown().map_err(|e| format!("{:?}", e))?;
        Ok(())
    }

    fn handle_message(&mut self, msg: &Message) -> Message {
        if msg.version != IPC_VERSION {
            return Message::error_response(&msg.id, &msg.route, "protocol_error", "bad version");
        }
        let reply_route = format!("{}.reply", msg.route);

        let result = match msg.direction {
            crate::ipc::message::Direction::Request => self.handle_request(msg),
            crate::ipc::message::Direction::Response => {
                Ok(json_object(vec![("acked".into(), jbool(true))]))
            }
            crate::ipc::message::Direction::Notification => {
                // e.g. "ui.reload" — core keeps sessions alive (TERM-003).
                Ok(json_object(vec![("acked".into(), jbool(true))]))
            }
        };

        match result {
            Ok(payload) => Message::response(&msg.id, &reply_route, payload),
            Err(e) => Message::error_response(&msg.id, &msg.route, "host_error", &e),
        }
    }

    fn handle_request(&mut self, msg: &Message) -> Result<JsonValue, String> {
        match msg.route.as_str() {
            "host.status" => self.handle_status(),
            "host.terminal.create" => self.handle_terminal_create(),
            "host.terminal.read" => self.handle_terminal_read(&msg.payload),
            "host.app.list" => self.handle_app_list(),
            "host.app.launch" => self.handle_app_launch(&msg.payload),
            "host.browser.open" => self.handle_browser_open(&msg.payload),
            "host.browser.engine" => self.handle_browser_engine(),
            "host.browser.capabilities" => self.handle_browser_capabilities(),
            "host.shutdown" => Ok(object(vec![])),
            _ => Err(format!("unknown route: {}", msg.route)),
        }
    }

    fn handle_status(&self) -> Result<JsonValue, String> {
        Ok(json_object(vec![
            ("core".into(), jstr("running")),
            ("terminal".into(), jstr("available")),
            ("ui".into(), jstr("connected")),
        ]))
    }

    fn handle_terminal_create(&mut self) -> Result<JsonValue, String> {
        let id = SessionId(format!("pty-{}", next_id()));
        self.sessions
            .create_terminal(self.terminal.as_ref(), id.clone())
            .map_err(|e| format!("{:?}", e))?;
        Ok(json_object(vec![("session_id".into(), jstr(&id.0))]))
    }

    fn handle_terminal_read(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let session_id = payload_string(payload, "session_id")?;
        let pty_session = self
            .sessions
            .pty(&SessionId(session_id))
            .ok_or("session not found")?
            .clone();
        let output = self
            .terminal
            .read_output(&pty_session)
            .map_err(|e| format!("{:?}", e))?;
        Ok(json_object(vec![("output".into(), jstr(&output))]))
    }

    fn handle_app_list(&self) -> Result<JsonValue, String> {
        let sessions_list: Vec<String> = self
            .sessions
            .list()
            .iter()
            .map(|s| format!("{:?}", s.id.0))
            .collect();
        Ok(json_object(vec![(
            "sessions".into(),
            jarr(sessions_list.iter().map(|s| jstr(s)).collect()),
        )]))
    }

    fn handle_app_launch(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let id = payload_string(payload, "id")?;
        let req = SpawnRequest {
            executable: "sh".into(),
            args: vec!["-c".into(), format!("echo 'app {} running'", id)],
        };
        let _handle = self.process.spawn(req).map_err(|e| format!("{:?}", e))?;
        let session_id = SessionId(format!("app-{}", id));
        self.sessions
            .create_terminal(self.terminal.as_ref(), session_id.clone())
            .map_err(|e| format!("{:?}", e))?;
        Ok(json_object(vec![("id".into(), jstr(&id))]))
    }

    fn handle_browser_open(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let url = payload_string(payload, "url")?;
        let config = BrowserConfig {
            profile: "default".into(),
            url: Some(url.clone()),
            off_the_record: false,
        };
        let _session = self.browser.create_session(config).map_err(|e| format!("{:?}", e))?;
        Ok(json_object(vec![("url".into(), jstr(&url))]))
    }

    fn handle_browser_engine(&self) -> Result<JsonValue, String> {
        Ok(json_object(vec![
            ("engine".into(), jstr(self.browser.engine_name())),
            ("version".into(), jstr(self.browser.engine_version())),
            ("experimental".into(), jbool(self.browser.is_experimental())),
        ]))
    }

    fn handle_browser_capabilities(&self) -> Result<JsonValue, String> {
        let caps = self.browser.capabilities();
        Ok(json_object(vec![
            ("supports_js".into(), jbool(caps.supports_js)),
            ("supports_downloads".into(), jbool(caps.supports_downloads)),
            ("is_experimental".into(), jbool(caps.is_experimental)),
        ]))
    }
}

type JsonValue = crate::ipc::json::JsonValue;

fn json_object(entries: Vec<(String, JsonValue)>) -> JsonValue {
    object(entries)
}

fn jstr(v: &str) -> JsonValue {
    JsonValue::String(v.into())
}

fn jbool(b: bool) -> JsonValue {
    JsonValue::Bool(b)
}

fn jarr(items: Vec<JsonValue>) -> JsonValue {
    JsonValue::Array(items)
}

fn payload_string(payload: &JsonValue, key: &str) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(100);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{linux_pty::LinuxPtyAdapter, linux_process::LinuxProcessAdapter};
    use crate::browser::adapters::mock::MockBrowserEngineAdapter;
    use crate::application::session_service::SessionService;
    use crate::browser::ports::engine::BrowserEnginePort;
    use crate::ports::terminal::TerminalPort;

    #[test]
    fn handle_status_returns_running() {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let mut server = IpcServer::new(process, terminal, sessions, browser);

        let msg = Message::request("1", "host.status", object(vec![]));
        let reply = server.handle_message(&msg);
        assert_eq!(reply.direction, crate::ipc::message::Direction::Response);
        let payload = reply.payload;
        let core = crate::ipc::json::get(&payload, "core")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(core, "running");
    }

    #[test]
    fn handle_browser_engine_returns_mock() {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let mut server = IpcServer::new(process, terminal, sessions, browser);

        let msg = Message::request("2", "host.browser.engine", object(vec![]));
        let reply = server.handle_message(&msg);
        let payload = reply.payload;
        let engine = crate::ipc::json::get(&payload, "engine")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(engine, "mock");
        let experimental = crate::ipc::json::get(&payload, "experimental")
            .and_then(crate::ipc::json::as_bool)
            .unwrap();
        assert!(experimental);
    }

    #[test]
    fn handle_terminal_create_returns_session_id() {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let mut server = IpcServer::new(process, terminal, sessions, browser);

        let msg = Message::request("3", "host.terminal.create", object(vec![]));
        let reply = server.handle_message(&msg);
        let payload = reply.payload;
        let session_id = crate::ipc::json::get(&payload, "session_id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert!(session_id.starts_with("pty-"));
    }

    #[test]
    fn handle_unknown_route_errors() {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let mut server = IpcServer::new(process, terminal, sessions, browser);

        let msg = Message::request("9", "host.doesnotexist", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(matches!(reply.direction, crate::ipc::message::Direction::Response));
        assert!(reply.error.is_some());
    }

    #[test]
    fn message_loop_shutdown_route() {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let mut server = IpcServer::new(process, terminal, sessions, browser);

        let msg = Message::request("sh", "host.shutdown", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_none());
    }
}
