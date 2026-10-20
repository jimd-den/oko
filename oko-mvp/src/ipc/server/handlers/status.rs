//! Handler: host.status — core status.
//! Small, composable, single responsibility.

use crate::ipc::json::object;
use crate::ipc::server::Core;

pub fn handle(server: &Core) -> Result<crate::ipc::json::JsonValue, String> {
    Ok(object(vec![
        ("core".into(), str("running")),
        ("terminal".into(), str("available")),
        (
            "ui".into(),
            str(if server.desktop_mode {
                "desktop"
            } else {
                "terminal"
            }),
        ),
    ]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_returns_running_status() {
        let server = test_server();
        let result = handle(&server).unwrap();
        let core = crate::ipc::json::get(&result, "core")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(core, "running");
    }

    fn test_server() -> Core {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());

        Core::new(
            process,
            terminal,
            sessions,
            policy,
            browser,
        )
    }
}
