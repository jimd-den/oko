//! The host core, and the stdio server that drives it.
//!
//! `Core` is everything the host can do, with no opinion about how a message
//! arrives: process adapter, terminal adapter, sessions, capability policy,
//! browser engine and the Blink installer. `IpcServer` drives it over a
//! transport; `crate::ipc::http` drives the same core over loopback HTTP for the
//! desktop UI. One core, more than one transport, the same typed and versioned
//! messages (SRD §4.2).

use crate::application::session_service::SessionService;
use crate::browser::adapters::blink_install::BlinkInstaller;
use crate::browser::ports::engine::BrowserEnginePort;
use crate::domain::capability::CapabilityPolicy;
use crate::ipc::transport::{IpcTransport, MessageBus};
use crate::ports::process::ProcessPort;
use crate::ports::terminal::TerminalPort;

mod capabilities;
mod dispatch;
mod handlers;

pub struct Core {
    process: Box<dyn ProcessPort>,
    terminal: Box<dyn TerminalPort>,
    sessions: SessionService,
    policy: CapabilityPolicy,
    browser: Box<dyn BrowserEnginePort>,
    blink_installer: BlinkInstaller,
    desktop_mode: bool,
    /// Why the browser engine is unusable, if it failed to initialize.
    /// `None` means it initialized. The engine is an optional service, so this
    /// is recorded rather than fatal (SRD ENG-004, ENG-008, §4.2).
    engine_unavailable: Option<String>,
    /// The root the filesystem panel may read and write inside.
    workspace_root: std::path::PathBuf,
    /// The directory the HTTP transport serves; apps live in `ui_root/apps`.
    ui_root: std::path::PathBuf,
}

impl Core {
    pub fn new(
        process: Box<dyn ProcessPort>,
        terminal: Box<dyn TerminalPort>,
        sessions: SessionService,
        policy: CapabilityPolicy,
        browser: Box<dyn BrowserEnginePort>,
        blink_installer: BlinkInstaller,
    ) -> Self {
        Self {
            process,
            terminal,
            sessions,
            policy,
            browser,
            blink_installer,
            desktop_mode: false,
            engine_unavailable: None,
            workspace_root: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            ui_root: std::path::PathBuf::from("ui"),
        }
    }

    /// The directory the desktop shell is served from.
    pub fn with_ui_root(mut self, root: std::path::PathBuf) -> Self {
        self.ui_root = root;
        self
    }

    /// Confine the filesystem panel to `root`.
    pub fn with_workspace_root(mut self, root: std::path::PathBuf) -> Self {
        self.workspace_root = root;
        self
    }

    /// Start the optional services.
    ///
    /// Contract: the browser engine is optional. If it cannot initialize, the
    /// reason is recorded and the core keeps serving, because the terminal must
    /// not depend on the engine (SRD ENG-004, ENG-008, §4.2). The failure is
    /// reported on `host.browser.engine`; no other engine is substituted.
    pub fn start(&mut self) {
        if let Err(e) = self.browser.initialize() {
            self.engine_unavailable = Some(format!("{:?}", e));
        }
    }

    /// Best effort: a failing optional service must not block a clean exit.
    pub fn stop(&mut self) {
        let _ = self.browser.shutdown();
    }

    /// Handle one message. The transport decides how it arrived; this decides
    /// what it means.
    pub fn handle(&mut self, msg: &crate::ipc::message::Message) -> crate::ipc::message::Message {
        dispatch::handle_message(self, msg)
    }
}

pub struct IpcServer<T: IpcTransport> {
    bus: MessageBus<T>,
    core: Core,
}

impl<T: IpcTransport> IpcServer<T> {
    pub fn new(bus: MessageBus<T>, core: Core) -> Self {
        Self { bus, core }
    }

    /// Serve IPC messages until `host.shutdown` or end of input.
    pub fn run(&mut self) -> Result<(), String> {
        self.core.start();
        loop {
            let msg = match self.bus.recv() {
                Ok(m) => m,
                Err(e) if e == "eof" => break,
                Err(e) => {
                    let _ = self.bus.send(&crate::ipc::message::Message::error_response(
                        "0",
                        "host.error",
                        "io",
                        &e,
                    ));
                    continue;
                }
            };
            if msg.route == "host.shutdown" {
                let _ = self.bus.send(&crate::ipc::message::Message::response(
                    &msg.id,
                    &msg.route,
                    crate::ipc::json::object(vec![]),
                ));
                break;
            }
            let response = self.core.handle(&msg);
            let _ = self.bus.send(&response);
        }
        self.core.stop();
        Ok(())
    }
}

impl IpcServer<crate::ipc::transport::StdioTransport> {
    pub fn new_stdio(core: Core) -> Self {
        let bus = MessageBus::new(crate::ipc::transport::StdioTransport::new());
        Self::new(bus, core)
    }
}
