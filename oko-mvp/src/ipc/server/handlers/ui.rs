//! Handler: ui.mode.desktop.
//! Small composable function, single responsibility.

use crate::ipc::json::object;
use crate::ipc::server::Core;

pub fn handle_desktop(server: &mut Core) -> Result<crate::ipc::json::JsonValue, String> {
    server.desktop_mode = true;
    Ok(object(vec![("mode".into(), str("desktop"))]))
}

/// Open a URL in a web pane (SRD UI-002, §10.2).
///
/// The pane is rendered by whichever engine is displaying the shell, so this
/// does not need the host's own engine session. What it does own is the trust
/// boundary: the capability is checked before the handler runs, and the scheme
/// is checked here, so a pane cannot be pointed at something that runs.
pub fn handle_pane_open(
    _server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let url = payload_string(payload, "url")?;
    if !is_openable(&url) {
        return Err(format!(
            "refusing to open '{}': http, https or a host path",
            url
        ));
    }
    Ok(object(vec![("url".into(), str(&url))]))
}

/// A page this host serves, or a page from the web. Nothing else: no `file:`,
/// no `javascript:`, no data URLs (SRD SEC-005).
fn is_openable(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with('/')
}

fn payload_string(payload: &crate::ipc::json::JsonValue, key: &str) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pane_opens_a_host_page_and_a_web_page() {
        let mut server = test_server();
        for url in ["/apps/computer.html", "https://example.com"] {
            let payload = crate::ipc::json::object(vec![("url".into(), str(url))]);
            assert!(handle_pane_open(&mut server, &payload).is_ok(), "{}", url);
        }
    }

    #[test]
    fn a_pane_refuses_a_scheme_that_runs_something() {
        let mut server = test_server();
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<b>x",
        ] {
            let payload = crate::ipc::json::object(vec![("url".into(), str(url))]);
            assert!(handle_pane_open(&mut server, &payload).is_err(), "{}", url);
        }
    }

    #[test]
    fn handle_desktop_switches_mode() {
        let mut server = test_server();
        assert!(!server.desktop_mode);
        let result = handle_desktop(&mut server).unwrap();
        assert!(server.desktop_mode);
        let mode = crate::ipc::json::get(&result, "mode")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(mode, "desktop");
    }

    fn test_server() -> Core {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::blink_install::BlinkInstaller;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;
        use std::env::temp_dir;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::SystemConfigure);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-ui-test");
        let _ = std::fs::create_dir_all(root.join("toolchain"));

        Core::new(
            process,
            terminal,
            sessions,
            policy,
            browser,
            BlinkInstaller::new(&root),
        )
    }
}
