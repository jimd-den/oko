//! IPC server — main struct and message loop.
//! Wires host API, terminal sessions, process adapter, browser engine,
//! Blink installer and software renderer behind the message bus (SRD §4.2).

use crate::application::session_service::SessionService;
use crate::browser::adapters::blink_install::BlinkInstaller;
use crate::browser::ports::engine::BrowserEnginePort;
use crate::domain::capability::CapabilityPolicy;
use crate::ipc::transport::{IpcTransport, MessageBus};
use crate::ports::process::ProcessPort;
use crate::ports::terminal::TerminalPort;
use std::path::Path;

mod capabilities;
mod dispatch;
mod handlers;

pub struct IpcServer<T: IpcTransport> {
    bus: MessageBus<T>,
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
}

impl<T: IpcTransport> IpcServer<T> {
    pub fn new(
        bus: MessageBus<T>,
        process: Box<dyn ProcessPort>,
        terminal: Box<dyn TerminalPort>,
        sessions: SessionService,
        policy: CapabilityPolicy,
        browser: Box<dyn BrowserEnginePort>,
        blink_installer: BlinkInstaller,
    ) -> Self {
        Self {
            bus,
            process,
            terminal,
            sessions,
            policy,
            browser,
            blink_installer,
            desktop_mode: false,
            engine_unavailable: None,
        }
    }

    /// Serve IPC messages until `host.shutdown` or end of input.
    ///
    /// Contract: the browser engine is an optional service. If it cannot
    /// initialize, the host records why and keeps serving, because the terminal
    /// core must not depend on the engine (SRD ENG-004, ENG-008, §4.2). The
    /// failure is reported on `host.browser.engine`; no other engine is
    /// substituted for the requested one.
    pub fn run(&mut self) -> Result<(), String> {
        if let Err(e) = self.browser.initialize() {
            self.engine_unavailable = Some(format!("{:?}", e));
        }
        loop {
            let msg = match self.bus.recv() {
                Ok(m) => m,
                Err(e) if e == "eof" => break,
                Err(e) => {
                    let _ = self.bus.send(&crate::ipc::message::Message::error_response(
                        "0", "host.error", "io", &e,
                    ));
                    continue;
                }
            };
            if msg.route == "host.shutdown" {
                let _ = self.bus.send(&crate::ipc::message::Message::response(
                    &msg.id, &msg.route, crate::ipc::json::object(vec![]),
                ));
                break;
            }
            let response = dispatch::handle_message(self, &msg);
            let _ = self.bus.send(&response);
        }
        // Best effort: a failing optional service must not block a clean exit.
        let _ = self.browser.shutdown();
        Ok(())
    }
}

impl IpcServer<crate::ipc::transport::StdioTransport> {
    pub fn new_stdio(
        process: Box<dyn ProcessPort>,
        terminal: Box<dyn TerminalPort>,
        sessions: SessionService,
        policy: CapabilityPolicy,
        browser: Box<dyn BrowserEnginePort>,
        workspace_root: &Path,
    ) -> Self {
        let bus = MessageBus::new(crate::ipc::transport::StdioTransport::new());
        let blink_installer = BlinkInstaller::new(workspace_root);
        Self::new(bus, process, terminal, sessions, policy, browser, blink_installer)
    }
}
