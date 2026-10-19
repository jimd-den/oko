//! Browser-engine test doubles and a host built around one of them.
//!
//! A real engine cannot be linked in a test, so these doubles stand in for the
//! two ways an engine reaches the host without working: it is not in this build
//! at all, or it dies after starting. Each one keeps reporting the identity it
//! was given, so a silent substitution by another engine shows up in an
//! assertion instead of hiding.

use super::scripted_transport::ScriptedTransport;
use oko_core::adapters::{linux_process::LinuxProcessAdapter, linux_pty::LinuxPtyAdapter};
use oko_core::application::session_service::SessionService;
use oko_core::browser::adapters::blink_install::BlinkInstaller;
use oko_core::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::ipc::message::Message;
use oko_core::ipc::server::{Core, IpcServer};
use oko_core::ipc::transport::MessageBus;
use std::path::PathBuf;

const UNLINKED: &str = "embedder not linked";
const CRASHED: &str = "renderer process terminated";

/// An engine that cannot start: not compiled into this build, or compiled but
/// never linked against its upstream source tree.
pub struct UnavailableEngine {
    pub name: &'static str,
}

/// An engine that starts and then dies, the way a renderer process does.
pub struct CrashingEngine {
    pub name: &'static str,
}

impl BrowserEnginePort for UnavailableEngine {
    fn initialize(&self) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable(UNLINKED.into()))
    }
    fn shutdown(&self) -> Result<(), BrowserError> {
        Ok(())
    }
    fn engine_name(&self) -> &str {
        self.name
    }
    fn engine_version(&self) -> &str {
        "test-double"
    }
    fn is_experimental(&self) -> bool {
        true
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn create_session(&self, _config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        Err(BrowserError::EngineUnavailable(UNLINKED.into()))
    }
    fn navigate(&self, _session: &BrowserSession, _url: &str) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable(UNLINKED.into()))
    }
    fn close_session(&self, _session: &BrowserSession) -> Result<(), BrowserError> {
        Err(BrowserError::EngineUnavailable(UNLINKED.into()))
    }
    fn execute_script(
        &self,
        _session: &BrowserSession,
        _script: &str,
    ) -> Result<String, BrowserError> {
        Err(BrowserError::EngineUnavailable(UNLINKED.into()))
    }
    fn last_crash_reason(&self) -> Option<String> {
        None
    }
}

impl BrowserEnginePort for CrashingEngine {
    fn initialize(&self) -> Result<(), BrowserError> {
        Ok(())
    }
    fn shutdown(&self) -> Result<(), BrowserError> {
        Ok(())
    }
    fn engine_name(&self) -> &str {
        self.name
    }
    fn engine_version(&self) -> &str {
        "test-double"
    }
    fn is_experimental(&self) -> bool {
        true
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn create_session(&self, _config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        Err(BrowserError::Crash(CRASHED.into()))
    }
    fn navigate(&self, _session: &BrowserSession, _url: &str) -> Result<(), BrowserError> {
        Err(BrowserError::Crash(CRASHED.into()))
    }
    fn close_session(&self, _session: &BrowserSession) -> Result<(), BrowserError> {
        Err(BrowserError::Crash(CRASHED.into()))
    }
    fn execute_script(
        &self,
        _session: &BrowserSession,
        _script: &str,
    ) -> Result<String, BrowserError> {
        Err(BrowserError::Crash(CRASHED.into()))
    }
    fn last_crash_reason(&self) -> Option<String> {
        Some(CRASHED.into())
    }
}

/// Run the host with the capabilities a desktop session is expected to hold.
pub fn run_host(
    script: Vec<Message>,
    engine: Box<dyn BrowserEnginePort>,
) -> (ScriptedTransport, Result<(), String>) {
    run_host_with_policy(script, engine, desktop_policy())
}

/// Run the host with a policy of the test's choosing, so a test can prove that
/// an unpermitted call is refused by the host itself (SRD SEC-002, AT-005).
pub fn run_host_with_policy(
    script: Vec<Message>,
    engine: Box<dyn BrowserEnginePort>,
    policy: CapabilityPolicy,
) -> (ScriptedTransport, Result<(), String>) {
    let transport = ScriptedTransport::new(script);
    let bus = MessageBus::new(transport.clone());
    let core = Core::new(
        Box::new(LinuxProcessAdapter::new()),
        Box::new(LinuxPtyAdapter::new()),
        SessionService::new(),
        policy,
        engine,
        BlinkInstaller::new(&PathBuf::from("/nonexistent-oko-workspace")),
    );
    let mut server = IpcServer::new(bus, core);
    let result = server.run();
    (transport, result)
}

fn desktop_policy() -> CapabilityPolicy {
    CapabilityPolicy::new()
        .with(Capability::TerminalCreate)
        .with(Capability::BrowserNavigate)
}

/// A stand-in engine binary: it answers `--version` and then waits to be
/// stopped, so the process-embedder path can be exercised without the real
/// engine and without the network.
pub fn fake_engine_binary() -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("oko-fake-engine-{}", std::process::id()));
    std::fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'Version: Servo fake-test'; exit 0; fi\nsleep 30\n",
    )
    .expect("write fake engine");
    let mut perms = std::fs::metadata(&path)
        .expect("stat fake engine")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod fake engine");
    path
}
