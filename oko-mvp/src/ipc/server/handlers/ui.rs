//! Handler: ui.mode.desktop.
//! Small composable function, single responsibility.

use crate::ipc::json::object;
use crate::ipc::server::IpcServer;
use crate::ipc::transport::IpcTransport;

pub fn handle_desktop<T: IpcTransport>(
    server: &mut IpcServer<T>,
) -> Result<crate::ipc::json::JsonValue, String> {
    server.desktop_mode = true;
    Ok(object(vec![("mode".into(), str("desktop"))]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_desktop_switches_mode() {
        let mut server = test_server();
        assert!(!server.desktop_mode);
        let result = handle_desktop(&mut server).unwrap();
        assert!(server.desktop_mode);
        let mode = crate::ipc::json::get(&result, "mode").and_then(crate::ipc::json::as_str).unwrap();
        assert_eq!(mode, "desktop");
    }

    fn test_server() -> IpcServer<crate::ipc::transport::StdioTransport> {
        use crate::application::session_service::SessionService;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;
        use std::env::temp_dir;

        let process: Box<dyn ProcessPort> = Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(crate::adapters::linux_pty::LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::SystemConfigure);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-ui-test");
        let _ = std::fs::create_dir_all(root.join("toolchain"));

        IpcServer::new_stdio(
            process, terminal, sessions, policy, browser, &root,
        )
    }
}
