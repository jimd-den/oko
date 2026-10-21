//! Composition root shared by the host and the console session.
//!
//! Both binaries need the same core: platform process adapter, real PTY,
//! session service, capability policy and the selected browser engine. Keeping
//! that wiring here means `oko-host` and `oko-session` cannot drift into two
//! different definitions of what the computer is.

use crate::adapters::{linux_process::LinuxProcessAdapter, portable_pty::PortablePtyAdapter};
use crate::application::session_service::SessionService;
use crate::browser::adapters::select;
use crate::domain::capability::{Capability, CapabilityPolicy};
use crate::ipc::server::Core;
use crate::ports::terminal::TerminalPort;
use std::path::Path;

/// Build the core for `workspace_root`, serving `engine` by name.
///
/// Contract: the engine is optional. A missing or unknown engine yields an
/// adapter that reports itself unavailable; it never stops the terminal.
pub fn core(workspace_root: &Path, engine: &str) -> Core {
    Core::new(
        Box::new(LinuxProcessAdapter::new()),
        Box::new(PortablePtyAdapter::new()) as Box<dyn TerminalPort>,
        SessionService::new(),
        policy(),
        select::engine_for(engine, workspace_root),
    )
}

/// The capabilities a desktop session holds (SRD §8.3).
pub fn policy() -> CapabilityPolicy {
    CapabilityPolicy::new()
        .with(Capability::ProcessSpawn)
        .with(Capability::BrowserNavigate)
        .with(Capability::GraphicsCreate)
        .with(Capability::FilesystemRead)
        .with(Capability::FilesystemWrite)
        .with(Capability::SystemConfigure)
        .with(Capability::TerminalCreate)
}
