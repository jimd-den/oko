//! IPC server — wires host API, terminal sessions, process adapter, browser
//! engine, Blink installer and software renderer behind the message bus
//! (SRD §4.2 IPC boundary).

use crate::application::session_service::SessionService;
use crate::browser::adapters::blink_install::{BlinkInstaller, InstallResult, InstallState};
use crate::browser::ports::engine::{
    BrowserConfig, BrowserEnginePort,
};
use crate::browser::api::host_api::{
    validate_request, HostApiRequest,
};
use crate::domain::capability::{Capability, CapabilityPolicy};
use crate::domain::session::SessionId;
use crate::graphics::software::Framebuffer;
use crate::ipc::json::object;
use crate::ipc::message::{Direction, Message, IPC_VERSION};
use crate::ipc::transport::{IpcTransport, MessageBus};
use crate::ports::process::{ProcessPort, SpawnRequest};
use crate::ports::terminal::TerminalPort;
use std::fs;
use std::path::PathBuf;

/// The IPC server: one message loop, routes to host API / core services.
pub struct IpcServer<T: IpcTransport> {
    bus: MessageBus<T>,
    process: Box<dyn ProcessPort>,
    terminal: Box<dyn TerminalPort>,
    sessions: SessionService,
    policy: CapabilityPolicy,
    browser: Box<dyn BrowserEnginePort>,
    blink_installer: BlinkInstaller,
    desktop_mode: bool,
}

impl IpcServer<crate::ipc::transport::StdioTransport> {
    pub fn new(
        process: Box<dyn ProcessPort>,
        terminal: Box<dyn TerminalPort>,
        sessions: SessionService,
        browser: Box<dyn BrowserEnginePort>,
        workspace_root: &std::path::Path,
    ) -> Self {
        let policy = CapabilityPolicy::new()
            .with(Capability::ProcessSpawn)
            .with(Capability::BrowserNavigate)
            .with(Capability::GraphicsCreate)
            .with(Capability::FilesystemRead)
            .with(Capability::SystemConfigure)
            .with(Capability::TerminalCreate);
        Self {
            bus: MessageBus::new(crate::ipc::transport::StdioTransport::new()),
            process,
            terminal,
            sessions,
            policy,
            browser,
            blink_installer: BlinkInstaller::new(workspace_root),
            desktop_mode: false,
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
            Direction::Request => self.handle_request(msg),
            Direction::Response => {
                Ok(object(vec![("acked".into(), jbool(true))]))
            }
            Direction::Notification => {
                // e.g. "ui.reload" — core keeps sessions alive (TERM-003).
                Ok(object(vec![("acked".into(), jbool(true))]))
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
            "host.app.stop" => self.handle_app_stop(&msg.payload),
            "host.browser.open" => self.handle_browser_open(&msg.payload),
            "host.browser.engine" => self.handle_browser_engine(),
            "host.browser.capabilities" => self.handle_browser_capabilities(),
            "blink.install" => self.handle_blink_install(),
            "blink.status" => self.handle_blink_status(),
            "blink.fetch-checksum" => self.handle_blink_fetch_checksum(&msg.payload),
            "ui.mode.desktop" => self.handle_ui_mode_desktop(),
            "render.demo" => self.handle_render_demo(&msg.payload),
            "render.stats" => self.handle_render_stats(),
            "render.capture" => self.handle_render_capture(&msg.payload),
            "host.shutdown" => Ok(object(vec![])),
            _ => Err(format!("unknown route: {}", msg.route)),
        }
    }

    // ---------- core status ----------

    fn handle_status(&self) -> Result<JsonValue, String> {
        Ok(json_object(vec![
            ("core".into(), jstr("running")),
            ("terminal".into(), jstr("available")),
            ("ui".into(), jstr(if self.desktop_mode { "desktop" } else { "terminal" })),
        ]))
    }

    // ---------- terminal ----------

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

    // ---------- app lifecycle ----------

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

    fn handle_app_stop(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let id = payload_string(payload, "id")?;
        let session_id = SessionId(format!("app-{}", id));
        self.sessions.stop(&session_id);
        Ok(json_object(vec![("id".into(), jstr(&id))]))
    }

    // ---------- browser ----------

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

    // ---------- blink install ----------

    fn handle_blink_install(&mut self) -> Result<JsonValue, String> {
        // Capability check (SRD §8.3): system.configure required.
        let _ = validate_request(&HostApiRequest::BlinkInstall, &self.policy)
            .map_err(|e| format!("{:?}", e))?;

        match self.blink_installer.install() {
            InstallResult::Ok => Ok(json_object(vec![
                ("state".into(), jstr("installed")),
                ("engine".into(), jstr("blink")),
            ])),
            InstallResult::AlreadyInstalled => Ok(json_object(vec![
                ("state".into(), jstr("already-installed")),
                ("engine".into(), jstr("blink")),
            ])),
            InstallResult::Failed(e) => Err(e),
            InstallResult::NotInstalled => Err("blink not installed".into()),
        }
    }

    fn handle_blink_status(&self) -> Result<JsonValue, String> {
        let state: String = match self.blink_installer.status() {
            InstallState::NotInstalled => "not-installed".to_string(),
            InstallState::Downloaded => "downloaded".to_string(),
            InstallState::Built => "built".to_string(),
        };
        Ok(json_object(vec![("state".into(), jstr(&state))]))
    }

    fn handle_blink_fetch_checksum(&self, payload: &JsonValue) -> Result<JsonValue, String> {
        let _ = validate_request(
            &HostApiRequest::BlinkFetchChecksum { path: String::new() },
            &self.policy,
        )
        .map_err(|e| format!("{:?}", e))?;

        let path = payload_string(payload, "path")?;
        match self.blink_installer.fetch_checksum(PathBuf::from(path).as_path()) {
            InstallResult::Failed(msg) => {
                // fetch_checksum returns Failed with the digest in the message.
                // Parse the digest out for the response.
                if let Some(digest) = msg.split("is ").nth(1).and_then(|s| s.split(' ').next()) {
                    Ok(json_object(vec![("digest".into(), jstr(digest))]))
                } else {
                    Err(msg)
                }
            }
            _ => Err("unexpected".into()),
        }
    }

    // ---------- ui mode ----------

    fn handle_ui_mode_desktop(&mut self) -> Result<JsonValue, String> {
        let _ = validate_request(&HostApiRequest::UiModeDesktop, &self.policy)
            .map_err(|e| format!("{:?}", e))?;
        self.desktop_mode = true;
        Ok(json_object(vec![("mode".into(), jstr("desktop"))]))
    }

    // ---------- render demo / stats / capture (software fallback) ----------

    fn handle_render_demo(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let _ = validate_request(&HostApiRequest::RenderDemo, &self.policy)
            .map_err(|e| format!("{:?}", e))?;
        let message = payload_string(payload, "message").unwrap_or_else(|_| "oko".into());
        let mut fb = Framebuffer::new(640, 480);
        fb.render_demo(&message);
        let ppm = fb.to_ppm();
        // Capture to a temp path if requested, otherwise report stats.
        let path = payload_string(payload, "path").unwrap_or_else(|_| "/tmp/oko-render-demo.ppm".into());
        fs::write(&path, &ppm).map_err(|e| format!("write demo: {}", e))?;
        Ok(json_object(vec![
            ("output".into(), jstr(&format!("rendered {}x{} software demo -> {}", fb.width, fb.height, path))),
            ("path".into(), jstr(&path)),
            ("backend".into(), jstr("software")),
        ]))
    }

    fn handle_render_stats(&self) -> Result<JsonValue, String> {
        let _ = validate_request(&HostApiRequest::RenderStats, &self.policy)
            .map_err(|e| format!("{:?}", e))?;
        Ok(json_object(vec![
            ("stats".into(), jstr("software renderer: 640x480 demo framebuffer")),
            ("backend".into(), jstr("software")),
            ("accelerated".into(), jbool(false)),
        ]))
    }

    fn handle_render_capture(&mut self, payload: &JsonValue) -> Result<JsonValue, String> {
        let _ = validate_request(&HostApiRequest::RenderCapture { path: String::new() }, &self.policy)
            .map_err(|e| format!("{:?}", e))?;
        let path = payload_string(payload, "path")?;
        let mut fb = Framebuffer::new(1280, 720);
        fb.render_demo("oko render capture");
        let ppm = fb.to_ppm();
        fs::write(&path, &ppm).map_err(|e| format!("write capture: {}", e))?;
        Ok(json_object(vec![
            ("path".into(), jstr(&path)),
            ("backend".into(), jstr("software")),
        ]))
    }
}

// ---------- helpers ----------

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
    use crate::application::session_service::SessionService;
    use crate::browser::adapters::mock::MockBrowserEngineAdapter;
    use crate::browser::ports::engine::BrowserEnginePort;
    use crate::ports::terminal::TerminalPort;
    use std::env::temp_dir;

    fn test_server() -> IpcServer<crate::ipc::transport::StdioTransport> {
        let process: Box<dyn ProcessPort> = Box::new(LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-ipc-test");
        let _ = fs::create_dir_all(root.join("toolchain"));
        IpcServer::new(process, terminal, sessions, browser, &root)
    }

    #[test]
    fn handle_status_returns_running() {
        let mut server = test_server();
        let msg = Message::request("1", "host.status", object(vec![]));
        let reply = server.handle_message(&msg);
        assert_eq!(reply.direction, Direction::Response);
        let payload = reply.payload;
        let core = crate::ipc::json::get(&payload, "core")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(core, "running");
    }

    #[test]
    fn handle_browser_engine_returns_mock() {
        let mut server = test_server();
        let msg = Message::request("2", "host.browser.engine", object(vec![]));
        let reply = server.handle_message(&msg);
        let payload = reply.payload;
        let engine = crate::ipc::json::get(&payload, "engine")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(engine, "mock");
    }

    #[test]
    fn handle_terminal_create_returns_session_id() {
        let mut server = test_server();
        let msg = Message::request("3", "host.terminal.create", object(vec![]));
        let reply = server.handle_message(&msg);
        let payload = reply.payload;
        let session_id = crate::ipc::json::get(&payload, "session_id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert!(session_id.starts_with("pty-"));
    }

    #[test]
    fn handle_blink_status_not_installed() {
        let mut server = test_server();
        let msg = Message::request("4", "blink.status", object(vec![]));
        let reply = server.handle_message(&msg);
        let payload = reply.payload;
        let state = crate::ipc::json::get(&payload, "state")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(state, "not-installed");
    }

    #[test]
    fn handle_blink_install_fails_without_checksum() {
        let mut server = test_server();
        let msg = Message::request("5", "blink.install", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_some());
    }

    #[test]
    fn handle_ui_mode_desktop_switches_mode() {
        let mut server = test_server();
        let msg = Message::request("6", "ui.mode.desktop", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_none());
        let payload = reply.payload;
        let mode = crate::ipc::json::get(&payload, "mode")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(mode, "desktop");
        // status now reports desktop
        let msg2 = Message::request("7", "host.status", object(vec![]));
        let reply2 = server.handle_message(&msg2);
        let ui = crate::ipc::json::get(&reply2.payload, "ui")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(ui, "desktop");
    }

    #[test]
    fn handle_render_demo_software_fallback() {
        let mut server = test_server();
        let msg = Message::request("8", "render.demo", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_none());
        let payload = reply.payload;
        let backend = crate::ipc::json::get(&payload, "backend")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(backend, "software");
    }

    #[test]
    fn handle_render_capture_writes_file() {
        let mut server = test_server();
        let out = temp_dir().join("oko-render-capture-test.ppm");
        let _ = fs::remove_file(&out);
        let payload = object(vec![("path".into(), JsonValue::String(out.to_string_lossy().into()))]);
        let msg = Message::request("9", "render.capture", payload);
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_none());
        assert!(out.exists());
        let _ = fs::remove_file(&out);
    }

    #[test]
    fn handle_unknown_route_errors() {
        let mut server = test_server();
        let msg = Message::request("99", "host.doesnotexist", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_some());
    }

    #[test]
    fn message_loop_shutdown_route() {
        let mut server = test_server();
        let msg = Message::request("sh", "host.shutdown", object(vec![]));
        let reply = server.handle_message(&msg);
        assert!(reply.error.is_none());
    }
}
