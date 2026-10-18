//! Acceptance test AT-008: browser engine lifecycle.
//!
//! Pass condition: a browser engine that is unavailable or crashed must not
//! take down the terminal core, and the failure must be reported clearly
//! rather than silently replaced by another engine
//! (SRD ENG-004, ENG-008, and §4.2 "a failing optional service cannot prevent
//! the core terminal from starting").
//!
//! The host is driven through an in-memory IPC transport, so the whole message
//! loop runs without a terminal attached.

#[path = "../support/mod.rs"]
mod support;

use oko_core::adapters::{linux_process::LinuxProcessAdapter, linux_pty::LinuxPtyAdapter};
use oko_core::application::session_service::SessionService;
use oko_core::browser::adapters::blink_install::BlinkInstaller;
use oko_core::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::ipc::json::{self, object};
use oko_core::ipc::message::Message;
use oko_core::ipc::server::IpcServer;
use oko_core::ipc::transport::MessageBus;
use std::path::PathBuf;
use support::ScriptedTransport;

/// Stands in for an engine that cannot start: an unlinked Servo or Blink
/// build, or an engine this build does not contain. It keeps reporting its own
/// name so that a silent substitution by another engine is visible in a test.
struct UnavailableEngine {
    name: &'static str,
}

impl BrowserEnginePort for UnavailableEngine {
    fn initialize(&self) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable("embedder not linked".into()))
    }
    fn shutdown(&self) -> Result<(), BrowserError> {
        Ok(())
    }
    fn engine_name(&self) -> &str {
        self.name
    }
    fn engine_version(&self) -> &str {
        "unlinked"
    }
    fn is_experimental(&self) -> bool {
        true
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn create_session(&self, _config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        Err(BrowserError::EngineUnavailable("embedder not linked".into()))
    }
    fn navigate(&self, _session: &BrowserSession, _url: &str) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable("embedder not linked".into()))
    }
    fn close_session(&self, _session: &BrowserSession) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable("embedder not linked".into()))
    }
    fn execute_script(&self, _session: &BrowserSession, _script: &str) -> Result<String, BrowserError> {
        Err(BrowserError::EngineUnavailable("embedder not linked".into()))
    }
    fn last_crash_reason(&self) -> Option<String> {
        None
    }
}

fn run_host(script: Vec<Message>) -> (ScriptedTransport, Result<(), String>) {
    let transport = ScriptedTransport::new(script);
    let policy = CapabilityPolicy::new()
        .with(Capability::TerminalCreate)
        .with(Capability::BrowserNavigate);
    let bus = MessageBus::new(transport.clone());
    let mut server = IpcServer::new(
        bus,
        Box::new(LinuxProcessAdapter::new()),
        Box::new(LinuxPtyAdapter::new()),
        SessionService::new(),
        policy,
        Box::new(UnavailableEngine { name: "servo" }),
        BlinkInstaller::new(&PathBuf::from("/nonexistent-oko-workspace")),
    );
    let result = server.run();
    (transport, result)
}

#[test]
fn unavailable_browser_engine_does_not_stop_the_terminal_core() {
    let script = vec![
        Message::request("1", "host.status", object(vec![])),
        Message::request("2", "host.terminal.create", object(vec![])),
        Message::request("3", "host.shutdown", object(vec![])),
    ];

    let (transport, run) = run_host(script);

    assert!(run.is_ok(), "host exited with {:?} instead of serving", run);
    let status = transport.reply_for("host.status").expect("host.status must be answered");
    assert_eq!(json::get(&status, "core").and_then(json::as_str), Some("running"));
    let created = transport
        .reply_for("host.terminal.create")
        .expect("terminal must still be usable without a browser engine");
    assert!(json::get(&created, "session_id").is_some(), "reply was {:?}", created);
}

#[test]
fn unavailable_engine_is_reported_without_silent_fallback() {
    let script = vec![
        Message::request("1", "host.browser.engine", object(vec![])),
        Message::request("2", "host.shutdown", object(vec![])),
    ];

    let (transport, run) = run_host(script);

    assert!(run.is_ok(), "host exited with {:?} instead of serving", run);
    let reply = transport.reply_for("host.browser.engine").expect("engine must be reported");
    assert_eq!(
        json::get(&reply, "engine").and_then(json::as_str),
        Some("servo"),
        "the requested engine must not be silently replaced (ENG-008)"
    );
    assert_eq!(
        json::get(&reply, "state").and_then(json::as_str),
        Some("unavailable"),
        "reply was {:?}",
        reply
    );
    let reason = json::get(&reply, "reason").and_then(json::as_str).unwrap_or("");
    assert!(!reason.is_empty(), "an unavailable engine must say why (OBS-003)");
}
